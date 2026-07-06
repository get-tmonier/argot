import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — lintez les règles que personne n’a écrites',
    description:
      'argot est un linter de voix. Il apprend la voix de votre dépôt à partir de son propre historique git, puis signale les passages qui ne ressemblent à personne de votre équipe. Aucun modèle, aucun cloud, aucun GPU.',
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
      'argot apprend la voix de votre dépôt à partir de son propre historique git, puis signale les passages qui ne ressemblent à personne de votre équipe. [[Aucun modèle. Aucun cloud. Aucun GPU.]]',
    ctaPrimary: 'Lire la doc',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · binaire statique unique · macOS & Linux · zéro dépendance',
  },
  demo: {
    label: 'La deuxième question',
    title: 'Le type-checker demande si ça compile. argot demande si c’est le vôtre.',
    body: 'Les linters répondent à « est-ce valide ? » — jamais à « est-ce ainsi qu’on écrit ici ? ». Cela vivait en revue de code, jusqu’à ce qu’un LLM l’enterre sous cent PR propres et bien typées. [[argot repose la question.]]',
    caption:
      'Une vue à la Django dans un dépôt 100 % FastAPI — un framework que ce code n’a jamais importé. mypy et ruff passent ; aucun linter ne bronche. [[argot le signale en ~150 ms.]]',
    seeLive: 'Voyez-le sur de vrais dépôts — FastAPI, Saleor, Hono',
  },
  catches: {
    label: 'Ce qu’il détecte',
    title: 'Techniquement correct. Socialement étranger.',
    body: 'Ni un remplaçant d’ESLint, de ruff ou de votre type-checker — il attrape ce qu’ils ne savent pas formuler : une dépendance, une API ou un paradigme [[que le dépôt n’a jamais utilisés]]. Et il est honnête sur la seule limite qu’il ne franchit pas.',
    items: [
      {
        title: 'Une dépendance étrangère',
        desc: 'Un import que le dépôt n’a jamais utilisé. Le signal le plus net — et celui qu’argot détecte le plus fiablement.',
      },
      {
        title: 'Une API étrangère',
        desc: 'Un appel vers une bibliothèque que le code évite — un autre client HTTP, ORM ou sérialiseur que celui du reste du dépôt. Le signe, c’est l’appel, pas seulement l’import.',
      },
      {
        title: 'Un paradigme étranger',
        desc: 'Tout un idiome venu d’ailleurs — une vue à la Django, une route Flask, une validation à la main — dans un dépôt qui n’écrit jamais ainsi.',
      },
      {
        title: 'La limite qu’il ne franchit pas',
        desc: 'Une mauvaise exception alors que [[tout le vocabulaire est déjà le vôtre]] — un choix, pas un motif étranger. argot ne les détecte pas de façon fiable, ne s’y engage jamais, et le dit.',
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
        desc: 'Le signal pour lequel argot est conçu. Imports et API étrangers insérés dans de vrais fichiers, jugés par le binaire livré : [[522 sur 527]] détectés.',
      },
      {
        value: '0,23 %',
        title: 'fausses alertes sur de vraies modifications',
        desc: 'À quelle fréquence argot se déclenche sur le [[code existant]] de votre dépôt — en rejouant 27 dépôts qu’il n’a jamais vus. Chaque corpus reste ≤ 0,98 %.',
      },
      {
        value: '150 ms',
        title: 'pour vérifier un changement',
        desc: 'Sur un dépôt de 34 000 fichiers, CPU de portable — assez rapide pour un [[hook pre-commit]]. Le fit unique prend ~7 s. [[Pas de GPU, pas de cloud.]]',
      },
      {
        value: '10',
        title: 'langages, un seul binaire',
        desc: 'Python, TypeScript, Go, Rust, Java, C#, C, C++, Ruby, PHP — un [[binaire statique]]. Les monorepos mixtes ont [[un seuil par langage]].',
      },
    ],
    finePrint:
      'Protocole sans fuite (issue #92) : rappel mesuré sur des fixtures plantées dans de vrais fichiers et jugées par le binaire livré ; fausses alertes par holdout temporel avec intervalles de confiance bootstrap au niveau commit. Chiffres complets par dépôt et méthodologie sur la page benchmarks.',
  },
  setup: {
    label: 'Configuration en une commande',
    title: 'Du clone au calibrage, en une ligne.',
    body: 'argot init apprend votre dépôt et vous dit s’il est prêt — sans config, sans annotation. Dépôt en désordre ? argot init --suggest (ou un agent) repère les dossiers générés et vendorisés qui ne doivent pas le façonner. [[Le modèle est un artefact de build — argot le garde hors de git pour vous.]]',
    caption:
      'Une seule commande. Elle garde même le modèle reconstructible hors de votre historique git.',
    ctaLocal: 'Configurez-le avec un seul prompt',
    ctaCi: 'ou un seul prompt pour la CI',
  },
  agents: {
    label: 'Conçu pour les agents IA',
    title: 'Votre agent écrit le code. argot le garde dans la voix du dépôt.',
    body: 'La majorité du code qu’argot juge est désormais écrite par un agent — donnez-lui le garde-fou. Trois skills l’intègrent, [[consultatif, jamais bloquant]] ; MCP lui fournit les idiomes du dépôt avant qu’il écrive une ligne.',
    cards: [
      {
        title: 'argot-setup · local',
        desc: 'Calibre le modèle de voix, et repère ce qui ne doit pas la façonner.',
      },
      {
        title: 'argot-check · local',
        desc: 'Score la diff pendant que l’agent travaille — consultatif, jamais bloquant.',
      },
      {
        title: 'argot-ci · CI',
        desc: 'Branche l’action GitHub — un score de voix sur chaque PR, sans setup local.',
      },
      {
        title: 'MCP · voice_context',
        desc: 'Fournit les idiomes du dépôt avant que l’agent ne génère une ligne.',
      },
    ],
    caption:
      'En local ou en CI, il ne bloque jamais un commit ni ne réécrit votre code. Il signale — vous décidez.',
  },
  ciScore: {
    label: 'En CI, sans la friction',
    title: 'Un score de voix sur chaque PR. Jamais une porte de merge.',
    body: 'Comme un contrôle de sécurité, argot décore chaque PR d’un score visuel et des points chauds — [[consultatif par défaut]]. Intentionnel ? Un argot mute suffit à l’accepter, avec une trace d’audit versionnée. Le relecteur garde le dernier mot.',
    caption:
      'Le même score atterrit dans le résumé Actions, un commentaire de PR épinglé, et l’onglet Security.',
  },
  cta: {
    title: 'Ajoutez la couche qui manque à votre CI.',
    body: 'argot est MIT et en alpha. Calibrez-le sur votre dépôt en deux minutes, puis voyez ce qu’il signale.',
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
