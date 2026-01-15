const fs = require("fs");
const path = require("path");

const newVersion = process.argv[2];
if (!newVersion) {
  console.error("Usage: node update-deps.js <newVersion>");
  process.exit(1);
}

const cratesDir = path.join(__dirname, "crates");
const crates = ["sentinel", "cli"];

crates.forEach((crate) => {
  const cargoTomlPath = path.join(cratesDir, crate, "Cargo.toml");
  let content = fs.readFileSync(cargoTomlPath, "utf8");

  // Add version to path dependencies
  content = content.replace(
    /sentinel-crypto = \{ (version = "[^"]+",\s*)?path = "\.\.\/sentinel-crypto" \}/g,
    `sentinel-crypto = { version = "${newVersion}", path = "../sentinel-crypto" }`
  );

  if (crate === "cli") {
    content = content.replace(
      /sentinel = \{ (version = "[^"]+",\s*)?path = "\.\.\/sentinel" \}/g,
      `sentinel = { version = "${newVersion}", path = "../sentinel" }`
    );
  }

  fs.writeFileSync(cargoTomlPath, content);
  console.log(`Updated ${cargoTomlPath}`);
});
