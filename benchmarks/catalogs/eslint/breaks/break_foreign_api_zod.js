// Break: validates a fully resolved flat config object through a zod schema
// before the ConfigArray is assembled, reached with a bare 'z' namespace and
// NO import in the hunk. 'zod' is 0-usage in the eslint corpus — flat config is
// validated by ESLint's own hand-written schema in this very file, never a
// schema-validation library. MEDIUM: no foreign import — the unattested foreign
// namespace z.strictObject / z.record / .safeParse (0 sites elsewhere in lib/)
// must be caught by call-receiver.
const resolvedConfigSchema = z.strictObject({
	rules: z.record(z.string(), z.unknown()),
	plugins: z.record(z.string(), z.unknown()),
	languageOptions: z.record(z.string(), z.unknown()).optional(),
});

/**
 * Validates a fully resolved flat config object against the expected shape.
 * @param {Object} config The resolved flat config.
 * @returns {{ success: boolean }} The schema validation result.
 */
function validateResolvedConfig(config) {
	return resolvedConfigSchema.safeParse(config);
}
