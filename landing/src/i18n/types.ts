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
    readonly audit: string;
    readonly engine: string;
    readonly docs: string;
  };
  readonly hero: {
    readonly eyebrow: string;
    readonly titleLead: string;
    readonly titleGradient: string;
    readonly subtitle: string;
    readonly ctaPrimary: string;
    readonly ctaSetup: string;
    readonly proof: string;
    readonly speed: string;
    readonly speedValue: string;
    readonly ctaSecondary: string;
    readonly install: string;
    readonly installNote: string;
    readonly installAlt: string;
    readonly watchFilm: string;
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
  readonly trust: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    /// The four moves an agent makes on a failing test — name + tiny example.
    readonly moves: readonly { readonly name: string; readonly example: string }[];
    readonly caption: string;
  };
  readonly audit: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
  };
  readonly customRules: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    /// Three selling points (short title + one-line desc).
    readonly points: readonly Feature[];
    /// Link text to the custom-rules docs page.
    readonly cta: string;
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
    readonly caughtCta: string;
  };
  readonly setup: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
    readonly installLabel: string;
    readonly skillsLabel: string;
    readonly skillsIntro: string;
    readonly skillDescs: readonly string[];
    readonly pluginNote: string;
    readonly pluginCta: string;
    readonly ctaLocal: string;
    readonly ctaCi: string;
  };
  readonly ciScore: {
    readonly label: string;
    readonly title: string;
    readonly body: string;
    readonly caption: string;
    readonly badge: string;
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
    readonly privacy: string;
    readonly acknowledgements: string;
  };
}
