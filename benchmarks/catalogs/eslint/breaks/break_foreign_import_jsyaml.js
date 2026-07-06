const yaml = require("js-yaml");

// Break: loads supplementary rule overrides from a sibling YAML file bolted
// onto the discovered flat config — 'js-yaml' is 0-usage in the eslint
// corpus; config files are eslint.config.{js,mjs,cjs,ts,mts,cts} only (see
// FLAT_CONFIG_FILENAMES above), never YAML.
/**
 * Loads additional rule overrides from a sibling `.eslint-overrides.yml` file.
 * @param {string} configFilePath The path to the discovered flat config file.
 * @returns {Promise<Object>} The parsed override rules, or an empty object.
 */
async function loadYamlOverrides(configFilePath) {
	const overridesPath = configFilePath.replace(
		/\.[cm]?[jt]s$/u,
		".eslint-overrides.yml",
	);
	const contents = await fs.readFile(overridesPath, "utf8").catch(() => null);

	return contents ? yaml.load(contents) : {};
}
