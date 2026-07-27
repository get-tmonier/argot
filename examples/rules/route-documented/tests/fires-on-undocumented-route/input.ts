import { app } from "../server";

app.get("/users/:id", getUser);
app.post("/users/:id/avatar", uploadAvatar);
