import fs from "node:fs";

const f = process.argv[2];
const fd = fs.openSync(f, "r");
const head = Buffer.alloc(16);
fs.readSync(fd, head, 0, 16, 0);
const size = head.readUInt32LE(12);
const buf = Buffer.alloc(size);
fs.readSync(fd, buf, 0, size, 16);
const header = JSON.parse(buf.toString("utf8"));
const out = [];
function walk(node, prefix) {
  for (const [name, val] of Object.entries(node.files ?? {})) {
    const p = prefix + "/" + name;
    if (val.files) walk(val, p);
    else out.push(p + "\t" + (val.size ?? 0));
  }
}
walk(header, "");
const pat = process.argv[3] ? new RegExp(process.argv[3], "i") : null;
const lines = out.filter((l) => !pat || pat.test(l));
fs.writeFileSync(process.argv[4] ?? "asar-files.txt", lines.join("\n"));
console.log("total=" + out.length + " matched=" + lines.length);
