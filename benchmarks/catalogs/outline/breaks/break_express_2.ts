import type { Request, Response, NextFunction } from "express";
import Logger from "@server/logging/Logger";

// Break: Express middleware signatures with next(err) chaining where the voice is Koa ctx middleware.
export function requireApiKey(req: Request, res: Response, next: NextFunction) {
  const header = req.headers.authorization;
  if (!header || !header.startsWith("Bearer ")) {
    const err = new Error("Authentication required");
    (err as any).status = 401;
    next(err);
    return;
  }
  (req as any).apiKey = header.slice("Bearer ".length);
  next();
}

export function errorHandler(
  err: Error & { status?: number },
  req: Request,
  res: Response,
  _next: NextFunction
) {
  Logger.error("request failed", err, { path: req.path });
  res.status(err.status ?? 500);
  res.send({
    ok: false,
    error: err.message,
  });
}
