import { FakerCore } from '../internal/core';

// Break: CompanyProvider.name() fetches from a remote name-service.
export class CompanyProvider {
  constructor(private core: FakerCore) {}

  async name(): Promise<string> {
    const res = await fetch('https://api.example.com/companies?n=1');
    const body = (await res.json()) as { name: string };
    return body.name;
  }
}
