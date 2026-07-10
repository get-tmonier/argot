import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — détectez le code IA qui ne colle pas à votre dépôt',
    description:
      'argot est un garde-fou local pour le code écrit par IA. Il apprend les motifs de votre dépôt depuis son historique git, puis signale le code qui n’a pas sa place — une dépendance jamais utilisée, une fonction que vous avez déjà, du code au mauvais endroit, un import qui casse votre architecture. Propulsé par un modèle d’embeddings de code qui tourne sur votre machine. Sans LLM, sans cloud, sans GPU.',
  },
  nav: {
    demo: 'Démo',
    catches: 'Ce qu’il détecte',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Modèle d’embeddings local · binaire Rust unique · sans LLM',
    titleLead: 'Lintez les règles',
    titleGradient: 'que personne n’a écrites.',
    subtitle:
      'Il signale le code IA qui ne colle pas à votre dépôt — une dépendance jamais utilisée, une fonction que vous [[avez déjà]], du code au mauvais endroit, un import qui casse votre architecture.',
    ctaPrimary: 'Lire la doc',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · binaire statique unique · macOS · Linux · Windows · 100 % local',
    installAlt: 'ou installer sans npm',
  },
  demo: {
    label: 'La deuxième question',
    title: 'Le type-checker demande si ça compile. argot demande si c’est le vôtre.',
    body: 'Les linters demandent « est-ce valide ? » — jamais « est-ce ainsi qu’on écrit ici ? » Un LLM l’enterre sous des PR propres et bien typées. [[argot repose la question.]]',
    caption:
      'Deux vrais contrôles : un framework étranger, et une fonction que le dépôt [[avait déjà]]. mypy et ruff passent les deux — pas argot.',
    seeLive: 'Voyez-le sur de vrais dépôts',
  },
  catches: {
    label: 'Ce qu’il détecte',
    title: 'Ça compile. C’est typé. Ça ne colle toujours pas.',
    body: 'Quatre détecteurs, tous appris de votre historique git — rien de tout cela n’est visible pour ESLint, ruff ou un type-checker. Et une limite honnête qu’il ne franchit pas : un mauvais choix fait de mots qui sont [[déjà les vôtres]] est un choix, pas un motif étranger.',
    items: [
      {
        title: 'Étranger',
        desc: 'Une dépendance, une API ou un idiome [[que le dépôt n’a jamais utilisés]]. Le signal le plus net — le mieux détecté.',
      },
      {
        title: 'Redondant',
        desc: 'Une fonction qui [[réinvente ce que vous avez déjà]]. argot retrouve l’originale et vous montre où elle vit.',
      },
      {
        title: 'Mal placé',
        desc: 'Le bon code, rangé dans le [[mauvais paquet]] — ses plus proches voisins vivent ailleurs.',
      },
      {
        title: 'Architecture',
        desc: 'Un import interne qui [[inverse votre architecture]] — une frontière que ce dépôt ne franchit jamais. 96,8 % détectés, zéro faux positif.',
      },
    ],
  },
  proof: {
    label: 'Mesuré, pas promis',
    title: 'Des chiffres honnêtes, sans fuite par construction.',
    stats: [
      {
        value: '98 %',
        title: 'détection étrangère visible',
        desc: 'Quand le symbole étranger est [[visible]] dans le diff — un import ou un appel explicite — jugé par le binaire livré : 604 sur 618.',
      },
      {
        value: '96,8 %',
        title: 'violations d’architecture détectées',
        desc: 'Violations de layering plantées (244 sur 252) à [[zéro faux positif]] — 0 déclenchement sur 140 modifications de contrôle.',
      },
      {
        value: '0,22 %',
        title: 'fausses alertes sur de vraies modifications',
        desc: 'À quelle fréquence argot se déclenche sur le [[code existant]] — 49 hunks sur 22 785 ; 31 dépôts, chacun ≤ 1,17 %.',
      },
      {
        value: '150 ms',
        title: 'pour vérifier un changement',
        desc: 'Sur un dépôt de 34 000 fichiers, CPU de portable. Le fit unique prend ~7 s. [[Sans GPU, sans cloud.]]',
      },
    ],
    languages:
      'Un seul [[binaire statique]], 11 langages : Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP.',
    finePrint:
      'Sans fuite par construction : rappel sur des motifs étrangers plantés dans de vrais fichiers ; fausses alertes par holdout temporel. La seule chose qu’un modèle de voix ne peut structurellement pas voir — l’étranger masqué — est publiée sur la page benchmarks, pas cachée.',
    benchmarksCta: 'Tous les chiffres par dépôt →',
  },
  setup: {
    label: 'Configuration · conçu pour les agents',
    title: 'Un CLI que votre agent peut piloter.',
    body: 'Les skills lancent argot [[et apportent le jugement]] : /argot-setup lit votre dépôt pour décider ce qui ne doit pas façonner sa voix — un SDK vendorisé, un dossier généré — écrit un argot.toml, calibre, et vérifie la détection. Informatif, jamais bloquant.',
    installLabel: 'Ajoutez les skills — Claude Code, Cursor, 70+ agents',
    skillsIntro: 'quatre slash-commands que votre agent lance :',
    skillDescs: [
      'lit votre arbre, écrit argot.toml, vérifie la détection',
      'score chaque diff, signale l’étranger — ne bloque jamais',
      'examine une PR selon la voix de votre dépôt, sans checkout',
      'un score de voix non bloquant sur chaque PR',
    ],
    ctaLocal: 'Ou pilotez le CLI à la main',
    ctaCi: 'le guide CI',
    caption:
      'Les skills apportent le jugement « exclure ce qui n’est pas vous » ; le modèle calibré reste hors de votre historique git.',
  },
  ciScore: {
    label: 'En CI, sans la friction',
    title: 'Un score de voix sur chaque PR. Jamais une porte de merge.',
    body: 'Un score visuel et les points chauds sur chaque PR — [[non bloquant par défaut]]. Intentionnel ? Un argot mute, versionné comme trace d’audit.',
    caption: 'Atterrit dans le résumé Actions, un commentaire de PR épinglé, et l’onglet Security.',
  },
  cta: {
    title: 'Ajoutez la couche qui manque à votre CI.',
    body: 'MIT · alpha. Calibrez sur votre dépôt en deux minutes, puis voyez ce qu’il signale.',
    primary: 'Commencer',
    secondary: 'Voir sur GitHub',
  },
  footer: {
    tagline: 'Un linter de voix pour les règles non écrites.',
    builtBy: 'Créé par Damien Meur',
    docs: 'Docs',
    npm: 'npm',
  },
};

export default fr;
