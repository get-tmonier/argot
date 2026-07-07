import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — lintez les règles que personne n’a écrites',
    description:
      'argot est un garde-fou pour le code écrit par IA. Il apprend les motifs de votre dépôt depuis son historique git, puis signale les dépendances, API et constructions qu’il n’a jamais utilisées. Aucun modèle, aucun cloud, aucun GPU.',
  },
  nav: {
    demo: 'Démo',
    catches: 'Ce qu’il détecte',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Local · un seul binaire Rust · sans LLM',
    titleLead: 'Lintez les règles',
    titleGradient: 'que personne n’a écrites.',
    subtitle:
      'Il signale le code étranger à votre dépôt — les dépendances et idiomes qu’une IA écrit et que votre code n’a [[jamais utilisés]].',
    ctaPrimary: 'Lire la doc',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · binaire statique unique · macOS · Linux · Windows · zéro dépendance',
    installAlt: 'ou installer sans npm',
  },
  demo: {
    label: 'La deuxième question',
    title: 'Le type-checker demande si ça compile. argot demande si c’est le vôtre.',
    body: 'Les linters demandent « est-ce valide ? » — jamais « est-ce ainsi qu’on écrit ici ? » Un LLM l’enterre sous des PR propres et bien typées. [[argot repose la question.]]',
    caption:
      'Une vue à la Django dans un dépôt 100 % FastAPI. mypy et ruff passent — [[argot le signale en ~150 ms.]]',
    seeLive: 'Voyez-le sur de vrais dépôts',
  },
  catches: {
    label: 'Ce qu’il détecte',
    title: 'Techniquement correct. Socialement étranger.',
    body: 'Ce qu’ESLint, ruff et les type-checkers ne savent pas formuler : une dépendance, une API ou un paradigme [[que le dépôt n’a jamais utilisés]].',
    items: [
      {
        title: 'Une dépendance étrangère',
        desc: 'Un import que le dépôt n’a jamais utilisé. Le signal le plus net — le mieux détecté.',
      },
      {
        title: 'Une API étrangère',
        desc: 'Un appel vers une bibliothèque que le reste du code évite.',
      },
      {
        title: 'Un paradigme étranger',
        desc: 'Tout un idiome venu d’ailleurs — une vue Django dans un dépôt FastAPI.',
      },
      {
        title: 'La limite qu’il ne franchit pas',
        desc: 'Une mauvaise exception alors que [[tout le vocabulaire est déjà le vôtre]]. argot ne s’engage pas sur un choix — et le dit.',
      },
    ],
  },
  proof: {
    label: 'Mesuré, pas promis',
    title: 'Des chiffres honnêtes, sans fuite par construction.',
    stats: [
      {
        value: '99 %',
        title: 'détection étrangère visible',
        desc: 'Imports et API étrangers insérés dans de vrais fichiers, jugés par le binaire livré : [[522 sur 527]].',
      },
      {
        value: '0,23 %',
        title: 'fausses alertes sur de vraies modifications',
        desc: 'À quelle fréquence argot se déclenche sur le [[code existant]] — 27 dépôts, chacun ≤ 0,98 %.',
      },
      {
        value: '150 ms',
        title: 'pour vérifier un changement',
        desc: 'Sur un dépôt de 34 000 fichiers, CPU de portable. Le fit unique prend ~7 s. [[Sans GPU, sans cloud.]]',
      },
      {
        value: '11',
        title: 'langages, un seul binaire',
        desc: 'Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP — un seul [[binaire statique]].',
      },
    ],
    finePrint:
      'Protocole sans fuite (issue #92) : rappel mesuré sur des fixtures plantées dans de vrais fichiers et jugées par le binaire livré ; fausses alertes par holdout temporel avec intervalles de confiance bootstrap. Chiffres complets par dépôt sur la page benchmarks.',
  },
  setup: {
    label: 'Configuration · conçu pour les agents',
    title: 'Un CLI que votre agent peut piloter.',
    body: 'Les skills lancent argot [[et apportent le jugement]] : /argot-setup lit votre dépôt pour décider ce qui ne doit pas façonner sa voix — un SDK vendorisé, un dossier généré — écrit un .argotignore, calibre, et vérifie la détection. Consultatif, jamais bloquant.',
    installLabel: 'Ajoutez les skills — Claude Code, Cursor, 70+ agents',
    skillsIntro: 'trois slash-commands que votre agent lance :',
    skillDescs: [
      'lit votre arbre, écrit .argotignore, vérifie la détection',
      'score chaque diff, signale l’étranger — ne bloque jamais',
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
    body: 'Un score visuel et les points chauds sur chaque PR — [[consultatif par défaut]]. Intentionnel ? Un argot mute, versionné comme trace d’audit.',
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
