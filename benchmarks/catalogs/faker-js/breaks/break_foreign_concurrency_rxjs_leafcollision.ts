import { ModuleBase } from '../../internal/module-base';

// Break: rxjs.from / rxjs.map build a reactive pipeline over generated seeds — a foreign async-streams runtime whose import is assumed pre-existing.
export class StreamedPlaylistModule extends ModuleBase {
  scaledSeeds(count: number): unknown {
    const seeds = Array.from({ length: count }, () =>
      this.faker.number.int(255)
    );
    const stream = rxjs.from(seeds);
    return stream.pipe(rxjs.map((seed: number) => seed % 128));
  }
}
