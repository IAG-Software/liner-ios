import fs from "node:fs";
import path from "node:path";

// Usage: node asar-extract.mjs <asar> <entryPath> <outFile>
const asar = process.argv[2];
const entry = process.argv[3].replace(/\\/g, "/");
const outFile = process.argv[4];

const fd = fs.openSync(asar, "r");
const head = Buffer.alloc(16);
fs.readSync(fd, head, 0, 16, 0);
const headerSize = head.readUInt32LE(12);
const headerBuf = Buffer.alloc(headerSize);
fs.readSync(fd, headerBuf, 0, headerSize, 16);
const header = JSON.parse(headerBuf.toString("utf8"));
const baseOffset = 8 + head.readUInt32LE(4);

let node = header;
for (const part of entry.split("/").filter(Boolean)) {
  node = node.files?.[part];
  if (!node) {
    console.error("not found: " + entry);
    process.exit(2);
  }
}
if (node.files) {
  console.error("is a directory: " + entry);
  process.exit(3);
}
const buf = Buffer.alloc(node.size);
fs.readSync(fd, buf, 0, node.size, baseOffset + Number(node.offset));
fs.mkdirSync(path.dirname(outFile), { recursive: true });
fs.writeFileSync(outFile, buf);
console.log("wrote " + outFile + " (" + node.size + " bytes)");
