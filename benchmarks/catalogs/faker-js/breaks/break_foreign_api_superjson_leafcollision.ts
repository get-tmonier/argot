import { SimpleModuleBase } from '../../internal/module-base';

// Break: superjson.stringify / superjson.parse serialize a generated record — a foreign serializer whose import is assumed pre-existing.
export class SnapshotModule extends SimpleModuleBase {
  roundTrip(sample: Record<string, unknown>): Record<string, unknown> {
    const encoded = superjson.stringify(sample);
    return superjson.parse(encoded);
  }
}
