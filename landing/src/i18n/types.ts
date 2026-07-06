export interface Feature {
  readonly title: string;
  readonly desc: string;
}

export interface SiteContent {
  readonly meta: {
    readonly title: string;
    readonly description: string;
  };
  readonly nav: {
    readonly demo: string;
    readonly catches: string;
    readonly docs: string;
  };
  readonly hero: {
    readonly eyebrow: string;
    readonly titleLead: string;
    readonly titleGradient: string;
    readonly subtitle: string;
    readonly ctaPrimary: string;
    readonly ctaSecondary: string;
    readonly install: string;
    readonly installNote: string;
    readonly installAlt: string;
  };
  readonly demo: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
    readonly seeLive: string;
  };
  readonly catches: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly items: readonly Feature[];
  };
  readonly proof: {
    readonly label: string;
    readonly title: string;
    readonly stats: readonly {
      readonly value: string;
      readonly title: string;
      readonly desc: string;
    }[];
    readonly finePrint: string;
  };
  readonly setup: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
    readonly agentLane: string;
    readonly agentsLine: string;
    readonly skillDescs: readonly string[];
    readonly localLane: string;
    readonly ctaLocal: string;
    readonly ctaCi: string;
  };
  readonly ciScore: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
  };
  readonly cta: {
    readonly title: string;
    readonly body: string;
    readonly primary: string;
    readonly secondary: string;
  };
  readonly footer: {
    readonly tagline: string;
    readonly builtBy: string;
    readonly docs: string;
    readonly npm: string;
  };
}
