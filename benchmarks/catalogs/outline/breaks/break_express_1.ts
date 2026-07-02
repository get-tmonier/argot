import express, { Request, Response } from "express";
import { Event } from "@server/models";

const app = express();

// Break: Express (req, res) handler with res.status().json() where the voice is koa-router + ctx.
app.get("/api/events/recent", async (req: Request, res: Response) => {
  const teamId = req.query.teamId as string;
  if (!teamId) {
    res.status(400).json({ error: "teamId is required" });
    return;
  }

  const events = await Event.findAll({
    where: { teamId },
    order: [["createdAt", "DESC"]],
    limit: 25,
  });

  res.status(200).json({
    ok: true,
    data: events.map((event) => ({
      id: event.id,
      name: event.name,
      createdAt: event.createdAt,
    })),
  });
});

export default app;
