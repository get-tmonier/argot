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
    readonly engine: string;
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
    /// One tab per detector: the rule name and a one-line caption for its pane.
    readonly tabs: readonly {
      readonly id: string;
      readonly label: string;
      readonly caption: string;
    }[];
    readonly seeLive: string;
  };
  readonly replay: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
  };
  readonly engine: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly cards: readonly Feature[];
    /// The measured speed row (value + short label) and its source line.
    readonly stats: readonly { readonly value: string; readonly label: string }[];
    readonly finePrint: string;
  };
  readonly proof: {
    readonly label: string;
    readonly title: string;
    readonly stats: readonly {
      readonly value: string;
      readonly title: string;
      readonly desc: string;
    }[];
    readonly languages: string;
    readonly finePrint: string;
    readonly benchmarksCta: string;
  };
  readonly setup: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
    readonly installLabel: string;
    readonly skillsIntro: string;
    readonly skillDescs: readonly string[];
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
