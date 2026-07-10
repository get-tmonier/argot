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
      'L’IA écrit du code valide qui n’est pas [[le vôtre]]. argot apprend la voix de votre dépôt depuis son historique git — et signale ce qui n’a pas sa place, avant le merge.',
    ctaPrimary: 'Lire la doc',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · binaire statique unique · macOS · Linux · Windows · 100 % local',
    installAlt: 'ou installer sans npm',
  },
  demo: {
    label: 'La deuxième question',
    title: 'Le type-checker demande si ça compile. argot demande si c’est le vôtre.',
    body: 'mypy demande si ça compile. ruff demande si c’est propre. Aucun ne pose la question de la review : [[est-ce ainsi qu’on fait ici ?]] Chaque exemple ci-dessous passe les deux — argot attrape les quatre.',
    tabs: [
      {
        id: 'foreign-import',
        label: 'foreign-import',
        caption:
          'Une vue Django dans un dépôt entièrement FastAPI — du Python valide, mais un framework que ce dépôt n’a jamais importé. L’évidence montre ce que le dépôt utilise à la place.',
      },
      {
        id: 'redundant',
        label: 'redundant',
        caption:
          'Le dépôt a déjà cette fonction. argot nomme l’originale, où elle vit, et la proximité du doublon — utilisez-la au lieu de merger un jumeau.',
      },
      {
        id: 'misplaced',
        label: 'misplaced',
        caption:
          'De la logique de téléchargement rangée sous cli/commands — ses plus proches voisins vivent tous dans core/downloader. Le bon code, au mauvais endroit.',
      },
      {
        id: 'layering',
        label: 'layering',
        caption:
          'Dans ce dépôt, cli importe core — jamais l’inverse. Cet import inverse discrètement l’architecture ; argot signale l’arête elle-même.',
      },
    ],
    seeLive: 'Voyez-le sur de vrais dépôts',
  },
  catches: {
    label: 'Ce qu’il détecte',
    title: 'Ça compile. C’est typé. Ça ne colle toujours pas.',
    body: 'Quatre détecteurs, tous appris de votre historique git — rien de tout cela n’est visible pour ESLint, ruff ou un type-checker. Et une limite honnête qu’il ne franchit pas : un mauvais choix fait de mots qui sont [[déjà les vôtres]] est un choix, pas un motif étranger.',
    items: [
      {
        title: 'Votre agent vient d’importer un framework que ce dépôt n’a jamais touché.',
        desc: 'argot signale la dépendance et montre ce que le dépôt utilise [[à la place]].',
      },
      {
        title: 'Votre agent vient de merger une deuxième implémentation de slugify.',
        desc: 'argot retrouve l’originale et vous montre [[où elle vit déjà]].',
      },
      {
        title: 'Votre agent vient de ranger de la logique de téléchargement sous cli/commands.',
        desc: 'argot pointe vers [[l’endroit où vivent déjà ses plus proches voisins]].',
      },
      {
        title: 'Votre agent vient de laisser cli plonger directement dans core — à l’envers.',
        desc: 'argot signale [[l’arête qui inverse votre architecture]].',
      },
    ],
  },
  proof: {
    label: 'Mesuré, pas promis',
    title: 'Des chiffres honnêtes, sans fuite par construction.',
    stats: [
      {
        value: '98 %',
        title: 'motifs étrangers détectés',
        desc: 'Une dépendance ou une API que le dépôt n’utilise jamais : 604 sur 618 — en ne se déclenchant que sur [[0,22 % des vraies modifications]] (49 hunks sur 22 785 ; pire dépôt 1,17 %).',
      },
      {
        value: '94 %',
        title: 'réinventions détectées · médiane',
        desc: '85–100 % par dépôt : des réécritures fidèles des [[propres fonctions]] du dépôt, plantées comme du code neuf et retracées à l’originale. Faux déclenchement ≤ 2,8 % des hunks.',
      },
      {
        value: '96 %',
        title: 'mauvais placements détectés · médiane',
        desc: '86–99 % partout où le dépôt a une architecture séparable — et il [[s’abstient]] là où il n’y en a pas, au lieu de deviner.',
      },
      {
        value: '96,8 %',
        title: 'violations d’architecture détectées',
        desc: 'Inversions de layering : 244 sur 252 détectées à [[zéro faux positif]] — 0 déclenchement sur 140 modifications de contrôle.',
      },
    ],
    languages:
      'Un seul [[binaire statique]], 11 langages : Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP.',
    finePrint:
      'Sans fuite par construction : rappel sur des motifs étrangers plantés dans de vrais fichiers ; fausses alertes par holdout temporel. La seule chose qu’un modèle de voix ne peut structurellement pas voir — l’étranger masqué — est publiée sur la page benchmarks, pas cachée. Vitesse, mesurée sur FastAPI (1 100+ fichiers, CPU de portable) : check ~0,2 s (~0,6 s quand le diff définit de nouvelles fonctions) · premier fit ~25 s · refresh ~4 s.',
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
