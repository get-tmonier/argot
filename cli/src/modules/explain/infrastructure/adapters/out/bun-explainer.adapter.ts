import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { Effect, Layer, Schema } from 'effect';
import { engineCmd } from '#engine-cmd.ts';
import { handleUvStderr } from '#spawn-with-progress.ts';
import { Explainer } from '#modules/explain/application/ports/out/explainer.port.ts';
import {
  ClaudeExitNonZero,
  ClaudeResponseInvalid,
  ClaudeSpawnFailed,
  ExplainEngineExitNonZero,
  ExplainEngineSpawnFailed,
} from '#modules/explain/domain/errors.ts';

const EngineRecord = Schema.Struct({
  file_path: Schema.String,
  line: Schema.Number,
  commit: Schema.String,
  surprise: Schema.Number,
  percentile: Schema.Number,
  tag: Schema.String,
  hunk_text: Schema.String,
  style_examples: Schema.Array(Schema.String),
});

const Explanation = Schema.Struct({
  summary: Schema.String,
  issues: Schema.Array(Schema.String),
});

const ClaudeEnvelope = Schema.Struct({
  structured_output: Schema.NullOr(Explanation),
  result: Schema.String,
});

const CLAUDE_SCHEMA = JSON.stringify({
  type: 'object',
  properties: {
    summary: { type: 'string' },
    issues: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'issues'],
});

const callClaude = (
  record: typeof EngineRecord.Type,
  claudeModel: string,
): Effect.Effect<
  typeof Explanation.Type,
  ClaudeSpawnFailed | ClaudeExitNonZero | ClaudeResponseInvalid
> =>
  Effect.gen(function* () {
    const examples = record.style_examples.map((ex, i) => `[${i + 1}] ${ex}`).join('\n---\n');

    const prompt =
      `This codebase's typical style (lowest-surprise commits from training data):\n\n` +
      `<examples>\n${examples}\n</examples>\n\n` +
      `This hunk from ${record.file_path}:${record.line} scored p${record.percentile} surprise ` +
      `(${record.commit === 'workdir' ? 'working tree' : `commit ${record.commit}`}):\n\n` +
      `<hunk>\n${record.hunk_text}\n</hunk>\n\n` +
      `In 2-3 sentences, what specific style differences do you see? Be concrete about naming, ` +
      `structure, patterns, line length. Return JSON only.`;

    const raw = yield* Effect.callback<string, ClaudeSpawnFailed | ClaudeExitNonZero>((resume) => {
      let proc: ReturnType<typeof spawn>;
      try {
        proc = spawn(
          'claude',
          [
            '--print',
            '--output-format',
            'json',
            '--model',
            claudeModel,
            '--tools',
            '',
            '--json-schema',
            CLAUDE_SCHEMA,
            prompt,
          ],
          { stdio: ['ignore', 'pipe', 'pipe'] },
        );
      } catch (cause: unknown) {
        resume(Effect.fail(new ClaudeSpawnFailed({ cause })));
        return;
      }

      const chunks: Buffer[] = [];
      const stderrChunks: Buffer[] = [];
      proc.stdout!.on('data', (chunk: Buffer) => chunks.push(chunk));
      proc.stderr!.on('data', (chunk: Buffer) => stderrChunks.push(chunk));
      proc.on('error', (cause: unknown) => resume(Effect.fail(new ClaudeSpawnFailed({ cause }))));
      proc.on('close', (code: number | null) => {
        if (code === 0) {
          resume(Effect.succeed(Buffer.concat(chunks).toString('utf-8')));
        } else {
          const stderr = Buffer.concat(stderrChunks).toString('utf-8');
          resume(Effect.fail(new ClaudeExitNonZero({ exitCode: code ?? -1, stderr })));
        }
      });
    });

    const envelope = yield* Schema.decodeUnknownEffect(Schema.fromJsonString(ClaudeEnvelope))(
      raw,
    ).pipe(Effect.mapError((cause) => new ClaudeResponseInvalid({ raw, cause })));
    const explanation =
      envelope.structured_output ??
      (yield* Schema.decodeUnknownEffect(Schema.fromJsonString(Explanation))(envelope.result).pipe(
        Effect.mapError((cause) => new ClaudeResponseInvalid({ raw: envelope.result, cause })),
      ));
    return explanation;
  });

function formatSeparator(index: number, total: number, record: typeof EngineRecord.Type): string {
  const info = `── [${index}/${total}] ${record.file_path}:${record.line}  ${record.tag}  ${record.surprise.toFixed(4)}  ${record.commit} `;
  const padLen = Math.max(0, 80 - info.length);
  return info + '─'.repeat(padLen);
}

export const BunExplainerLive = Layer.effect(Explainer)(
  Effect.succeed({
    runExplain: ({
      repoPath,
      ref,
      modelPath,
      datasetPath,
      claudeModel,
      threshold,
    }: {
      repoPath: string;
      ref: string;
      modelPath: string;
      datasetPath: string;
      claudeModel: string;
      threshold: number;
    }) =>
      Effect.callback<void, ExplainEngineSpawnFailed | ExplainEngineExitNonZero>((resume) => {
        const { cmd, args } = engineCmd('argot.explain');
        let proc: ReturnType<typeof spawn>;
        try {
          proc = spawn(
            cmd,
            [
              ...args,
              repoPath,
              ref,
              '--model',
              modelPath,
              '--dataset',
              datasetPath,
              '--threshold',
              String(threshold),
            ],
            { stdio: ['ignore', 'pipe', 'pipe'] },
          );
        } catch (cause: unknown) {
          resume(Effect.fail(new ExplainEngineSpawnFailed({ cause })));
          return;
        }

        const stderrChunks: Buffer[] = [];
        const stopSpinner = handleUvStderr(proc.stderr!, (chunk) => stderrChunks.push(chunk));

        proc.on('error', (cause: unknown) =>
          resume(Effect.fail(new ExplainEngineSpawnFailed({ cause }))),
        );

        const items: Array<{
          record: typeof EngineRecord.Type;
          explanationPromise: Promise<typeof Explanation.Type>;
        }> = [];

        const rl = createInterface({ input: proc.stdout! });

        rl.on('line', (line: string) => {
          if (!line.trim()) return;
          let record: typeof EngineRecord.Type;
          try {
            record = Effect.runSync(
              Schema.decodeUnknownEffect(Schema.fromJsonString(EngineRecord))(line),
            );
          } catch {
            return;
          }
          items.push({
            record,
            explanationPromise: Effect.runPromise(callClaude(record, claudeModel)),
          });
        });

        proc.on('close', (code: number | null) => {
          Promise.all(items.map(({ explanationPromise }) => explanationPromise))
            .then((explanations) => {
              stopSpinner();
              const out = (s: string): void => {
                process.stdout.write(`${s}\n`);
              };
              if (items.length === 0) {
                out('No violations above threshold — nothing to explain.');
              } else {
                out(`\n${items.length} violation(s) above threshold — explaining...`);
                items.forEach(({ record }, i) => {
                  const explanation = explanations[i]!;
                  out('');
                  out(formatSeparator(i + 1, items.length, record));
                  out('');
                  out(`  ${explanation.summary}`);
                  if (explanation.issues.length > 0) {
                    out('');
                    for (const issue of explanation.issues) {
                      out(`  • ${issue}`);
                    }
                  }
                  out('');
                });
              }
              if (code === 0) {
                resume(Effect.void);
              } else {
                const stderr = Buffer.concat(stderrChunks).toString('utf-8');
                resume(Effect.fail(new ExplainEngineExitNonZero({ exitCode: code ?? -1, stderr })));
              }
            })
            .catch((cause: unknown) => {
              stopSpinner();
              resume(
                Effect.fail(
                  new ExplainEngineExitNonZero({
                    exitCode: code ?? -1,
                    stderr: String(cause),
                  }),
                ),
              );
            });
        });
      }),
  }),
);
