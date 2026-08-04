import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";

const temporaryDirectory = mkdtempSync(path.join(tmpdir(), "eth-prices-package-"));

try {
  execFileSync("pnpm", ["pack", "--pack-destination", temporaryDirectory], { stdio: "inherit" });
  const tarball = readdirSync(temporaryDirectory).find(file => file.endsWith(".tgz"));

  if (tarball === undefined) throw new Error("pnpm pack did not produce a tarball");

  const installDirectory = path.join(temporaryDirectory, "install");

  execFileSync("npm", ["install", "--ignore-scripts", "--prefix", installDirectory, path.join(temporaryDirectory, tarball)], {
    stdio: "inherit",
  });

  if (!existsSync(path.join(installDirectory, "node_modules", "eth-prices", "LICENSE"))) {
    throw new Error("Published package is missing LICENSE");
  }

  writeFileSync(
    path.join(installDirectory, "smoke.mjs"),
    "import { createRouter } from 'eth-prices';\nif (typeof createRouter !== 'function') process.exit(1);\n",
  );
  execFileSync(process.execPath, ["smoke.mjs"], { cwd: installDirectory, stdio: "inherit" });
}
finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}
