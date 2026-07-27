import { app } from "../server";

app.get("/users/:id", getUser);
