import axios from "axios";
import { action, observable } from "mobx";

// Break: axios import at line 1 where every network call goes through the shared ApiClient util.
export default class StarredDocumentsStore {
  @observable
  documentIds: string[] = [];

  @action
  async fetchStarred() {
    const response = await axios.post(
      "/api/stars.list",
      {},
      {
        headers: { "Content-Type": "application/json" },
        withCredentials: true,
      }
    );
    this.documentIds = response.data.data.map(
      (star: { documentId: string }) => star.documentId
    );
  }

  @action
  async star(documentId: string) {
    await axios.post("/api/stars.create", { documentId });
    this.documentIds.push(documentId);
  }
}
