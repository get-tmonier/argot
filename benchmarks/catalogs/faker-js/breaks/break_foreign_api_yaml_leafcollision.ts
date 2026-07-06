import { ModuleBase } from '../../internal/module-base';

// Break: YAML.stringify / YAML.parse (the yaml package) serialize a generated config blob — a foreign serializer whose import is assumed pre-existing.
export class ConfigBlobModule extends ModuleBase {
  configBlob(): string {
    const cfg = {
      host: this.faker.internet.domainName(),
      port: this.faker.number.int({ min: 1024, max: 65535 }),
    };
    const text = YAML.stringify(cfg);
    return YAML.parse(text).host;
  }
}
