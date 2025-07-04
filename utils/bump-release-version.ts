// usage: bun run utils/bump-release-version.ts bump [--major|--minor|--patch]

const { program } = require("commander");

const files = {
  js: {
    cargoToml: Bun.file("./topk-js/Cargo.toml"),
    packageJson: Bun.file("./topk-js/package.json"),
  },
  py: {
    cargoToml: Bun.file("./topk-py/Cargo.toml"),
  },
  rs: {
    cargoToml: Bun.file("./topk-rs/Cargo.toml"),
  },
};

// Get versions
async function getVersions() {
  const versions = {
    js: {
      packageJson: await getPackageJsonVersion(files.js.packageJson),
      cargoToml: await getCargoTomlVersion(files.js.cargoToml),
    },
    py: {
      cargoToml: await getCargoTomlVersion(files.py.cargoToml),
    },
    rs: {
      cargoToml: await getCargoTomlVersion(files.rs.cargoToml),
    },
  };

  const uniqueVersions = new Set([
    versions.js.packageJson,
    versions.js.cargoToml,
    versions.py.cargoToml,
    versions.rs.cargoToml,
  ]);

  return { versions, consistent: uniqueVersions.size === 1 };
}

async function getCargoTomlVersion(file: Bun.File) {
  return Bun.TOML.parse(await file.text())["package"]["version"];
}

async function getPackageJsonVersion(file: Bun.File) {
  return JSON.parse(await file.text())["version"];
}

// Update versions
async function updateVersions(version: string) {
  await updateCargoTomlVersion(files.js.cargoToml, version);
  await updatePackageJsonVersion(files.js.packageJson, version);
  await updateCargoTomlVersion(files.py.cargoToml, version);
  await updateCargoTomlVersion(files.rs.cargoToml, version);
}

async function updateCargoTomlVersion(file: Bun.File, version: string) {
  const text = await file.text();
  const updated = text.replace(/version = ".*"/, `version = "${version}"`);
  await Bun.write(file, updated);
}

async function updatePackageJsonVersion(file: Bun.File, version: string) {
  const text = await file.text();
  const updated = text.replace(/"version": ".*"/, `"version": "${version}"`);
  await Bun.write(file, updated);
}

program
  .name("sdk-bump-version-util")
  .description("CLI to bump versions of the Topk SDKs")
  .version("0.8.0");

program
  .command("split")
  .description("Split a string into substrings and display as an array")
  .argument("<string>", "string to split")
  .option("--first", "display just the first substring")
  .option("-s, --separator <char>", "separator character", ",")
  .action((str, options) => {
    const limit = options.first ? 1 : undefined;
    console.log(str.split(options.separator, limit));
  });

program.command("check").action(async () => {
  const { versions, consistent } = await getVersions();

  console.log(`topk-rs/Cargo.toml\t${versions.rs.cargoToml}`);
  console.log(`topk-py/Cargo.toml\t${versions.py.cargoToml}`);
  console.log(`topk-js/Cargo.toml\t${versions.js.cargoToml}`);
  console.log(`topk-js/package.json\t${versions.js.packageJson}`);

  if (!consistent) {
    program.error("ERROR: versions are not consistent");
  }
});

program
  .command("update")
  .argument("<version>", "version to update to")
  .action(async (version) => {
    await updatePackageJsonVersion(files.js.packageJson, version);
    await updateCargoTomlVersion(files.js.cargoToml, version);
    await updateCargoTomlVersion(files.py.cargoToml, version);
    await updateCargoTomlVersion(files.rs.cargoToml, version);

    console.info(`updated all versions to ${version}`);
  });

program
  .command("bump")
  .option("--major", "bump the major version")
  .option("--minor", "bump the minor version")
  .option("--patch", "bump the patch version", true)
  .action(async (options) => {
    let { versions, consistent } = await getVersions();
    if (!consistent) {
      program.error("ERROR: versions are not consistent");
    }
    let version = versions.js.packageJson; // arbitrary
    console.info(`current version\t${version}`);

    let [major, minor, patch] = version.split(".").map(Number);
    if (options.major) {
      major++;
      minor = 0;
      patch = 0;
    } else if (options.minor) {
      minor++;
      patch = 0;
    } else if (options.patch) {
      patch++;
    }

    let newVersion = `${major}.${minor}.${patch}`;
    console.info(`new version\t${newVersion}`);

    await updateVersions(newVersion);
  });
program.parse();
