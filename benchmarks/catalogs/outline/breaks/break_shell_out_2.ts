import { spawn } from "child_process";
import Logger from "@server/logging/Logger";

// Break: spawning pg_dump with stdio event callbacks in a queue task; tasks operate on models, not processes.
export function backupTeamDatabase(teamId: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const outfile = `/tmp/backup-${teamId}.sql`;
    const child = spawn("pg_dump", [
      "--table=documents",
      "--table=collections",
      `--file=${outfile}`,
      process.env.DATABASE_URL ?? "",
    ]);

    let stderr = "";

    child.stderr.on("data", (chunk: Buffer) => {
      stderr += chunk.toString();
    });

    child.on("close", (code: number) => {
      if (code === 0) {
        Logger.info("task", `backup written to ${outfile}`);
        resolve(outfile);
      } else {
        reject(new Error(`pg_dump exited with ${code}: ${stderr}`));
      }
    });

    child.on("error", (err: Error) => {
      reject(err);
    });
  });
}
