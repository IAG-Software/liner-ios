<#
.SYNOPSIS
    Render a URL with exact iPhone device metrics in headless Chrome, evaluate JS,
    and optionally save a screenshot.

.DESCRIPTION
    Uses the DevTools Protocol's Emulation.setDeviceMetricsOverride so the CSS
    viewport is exactly Width x Height at the requested device pixel ratio — the
    same thing Chrome's device toolbar does. `--window-size` alone does not do
    this correctly once a scale factor is involved.

.PARAMETER Url
    Page to load.

.PARAMETER Expression
    JavaScript to evaluate; should produce a JSON-serialisable value.

.PARAMETER Width / Height
    CSS viewport (iPhone 14 Pro = 393x852, 15 Pro Max = 430x932).

.PARAMETER Scale
    Device pixel ratio (3 for modern iPhones).

.PARAMETER Screenshot
    Optional PNG output path.

.EXAMPLE
    & scripts/render-check.ps1 -Expression "innerWidth" -Screenshot out.png
#>
[CmdletBinding()]
param(
    [string]$Url = "http://localhost:5173/",
    [string]$Expression = "JSON.stringify({vw: innerWidth, dpr: devicePixelRatio})",
    [int]$Width = 393,
    [int]$Height = 852,
    [int]$Scale = 3,
    [string]$Screenshot,
    [int]$SettleMs = 6000
)

$ErrorActionPreference = "Stop"
$chrome = "C:\Program Files\Google\Chrome\Application\chrome.exe"
if (-not (Test-Path $chrome)) { throw "chrome not found at $chrome" }

$port = Get-Random -Minimum 9300 -Maximum 9800
$profile = Join-Path $env:TEMP ("liner-cdp-" + [guid]::NewGuid().ToString("N"))

$proc = Start-Process -FilePath $chrome -PassThru -ArgumentList @(
    "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
    "--remote-debugging-port=$port", "--user-data-dir=$profile",
    "--window-size=$Width,$Height", "about:blank"
)

$ws = $null
try {
    $page = $null
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep -Milliseconds 250
        try {
            $targets = Invoke-RestMethod "http://127.0.0.1:$port/json/list" -TimeoutSec 2
            $page = $targets | Where-Object { $_.type -eq "page" } | Select-Object -First 1
            if ($page) { break }
        } catch { }
    }
    if (-not $page) { throw "no page target on port $port" }

    $ws = [System.Net.WebSockets.ClientWebSocket]::new()
    $ws.ConnectAsync([Uri]$page.webSocketDebuggerUrl, [Threading.CancellationToken]::None).Wait()

    $script:nextId = 0
    function Send-Cdp([string]$method, $params) {
        $script:nextId++
        $id = $script:nextId
        $json = @{ id = $id; method = $method; params = $params } | ConvertTo-Json -Depth 10 -Compress
        $bytes = [Text.Encoding]::UTF8.GetBytes($json)
        $ws.SendAsync([ArraySegment[byte]]::new($bytes), [System.Net.WebSockets.WebSocketMessageType]::Text, $true, [Threading.CancellationToken]::None).Wait()

        while ($true) {
            $buffer = New-Object byte[] 524288
            $seg = [ArraySegment[byte]]::new($buffer)
            $sb = [Text.StringBuilder]::new()
            do {
                $res = $ws.ReceiveAsync($seg, [Threading.CancellationToken]::None).Result
                $sb.Append([Text.Encoding]::UTF8.GetString($buffer, 0, $res.Count)) | Out-Null
            } while (-not $res.EndOfMessage)

            $message = $sb.ToString() | ConvertFrom-Json
            # skip protocol events; wait for the reply carrying our id
            if ($message.PSObject.Properties.Name -contains "id" -and $message.id -eq $id) {
                return $message
            }
        }
    }

    # exact iPhone metrics, equivalent to DevTools' device toolbar
    Send-Cdp "Emulation.setDeviceMetricsOverride" @{
        width = $Width; height = $Height
        deviceScaleFactor = $Scale; mobile = $true
    } | Out-Null
    Send-Cdp "Page.enable" @{} | Out-Null
    Send-Cdp "Page.navigate" @{ url = $Url } | Out-Null
    Start-Sleep -Milliseconds $SettleMs

    $reply = Send-Cdp "Runtime.evaluate" @{
        expression = $Expression; returnByValue = $true; awaitPromise = $true
    }
    $result = $reply.result.result
    if ($result.exceptionDetails) { Write-Host "JS ERROR: $($result.exceptionDetails.text)" }

    $value = $result.value
    if ($value -is [string]) {
        $parsed = $null
        try { $parsed = $value | ConvertFrom-Json } catch { }
        if ($null -ne $parsed) { $parsed | ConvertTo-Json -Depth 10 } else { $value }
    } elseif ($null -ne $value) {
        $value | ConvertTo-Json -Depth 10
    }

    if ($Screenshot) {
        $shot = Send-Cdp "Page.captureScreenshot" @{ format = "png"; captureBeyondViewport = $false }
        [IO.File]::WriteAllBytes($Screenshot, [Convert]::FromBase64String($shot.result.data))
        Write-Host "screenshot: $Screenshot ($Width x $Height @${Scale}x)"
    }

    $ws.CloseAsync([System.Net.WebSockets.WebSocketCloseStatus]::NormalClosure, "done", [Threading.CancellationToken]::None).Wait()
}
finally {
    if ($ws) { $ws.Dispose() }
    if ($proc -and -not $proc.HasExited) { $proc.Kill() }
    Start-Sleep -Milliseconds 300
    Remove-Item $profile -Recurse -Force -ErrorAction SilentlyContinue
}
