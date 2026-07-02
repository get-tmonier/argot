import { execSync } from "child_process";
import Router from "koa-router";
import auth from "@server/middlewares/authentication";
import type { APIContext } from "@server/types";

const router = new Router();

// Break: synchronous execSync shell-out from a request handler; child_process is absent from production.
router.post("attachments.optimize", auth(), async (ctx: APIContext) => {
  const { filePath } = ctx.request.body as { filePath: string };

  const output = execSync(`convert ${filePath} -strip -quality 82 ${filePath}.opt`, {
    encoding: "utf8",
    timeout: 30000,
  });

  const stats = execSync(`stat -f %z ${filePath}.opt`, { encoding: "utf8" });

  ctx.body = {
    data: {
      output: output.trim(),
      sizeBytes: parseInt(stats.trim(), 10),
    },
  };
});

export default router;
