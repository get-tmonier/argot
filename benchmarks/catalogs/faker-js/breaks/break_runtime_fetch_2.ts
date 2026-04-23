import { FakerCore } from '../internal/core';

// Break: ImageProvider.url() issues a live network fetch.
export class ImageProvider {
  constructor(private core: FakerCore) {}

  async url(width = 640, height = 480): Promise<string> {
    const res = await fetch(`https://picsum.photos/${width}/${height}`);
    return res.url;
  }
}
