// Break: zod schema .parse() run-config validation instead of Dagit's GraphQL-typed config + yaml package.
// Dagit validates launch config through the server's GraphQL schema and parses YAML with the `yaml` package;
// a zod schema (z.object(...).parse()) is a client-side runtime-validation library that ui-core never imports.
import {z} from 'zod';

const LaunchConfigSchema = z.object({
  jobName: z.string().min(1),
  runConfigYaml: z.string(),
  tags: z.record(z.string()).default({}),
  mode: z.enum(['default', 'test']).optional(),
});

export type LaunchConfig = z.infer<typeof LaunchConfigSchema>;

export function parseLaunchConfig(raw: unknown): LaunchConfig {
  return LaunchConfigSchema.parse(raw);
}

export function safeParseLaunchConfig(raw: unknown): LaunchConfig | null {
  const result = LaunchConfigSchema.safeParse(raw);
  return result.success ? result.data : null;
}
