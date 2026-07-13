import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — détectez le code IA qui ne colle pas à votre dépôt',
    description:
      'Un garde-fou local pour le code écrit par IA. argot apprend votre dépôt depuis son historique git et signale ce qui n’a pas sa place — dépendances étrangères, fonctions réinventées, architecture inversée, tests truqués — plus les conventions que vous scriptez vous-même. Sans LLM, sans cloud, sans GPU.',
  },
  nav: {
    demo: 'Démo',
    audit: 'Audit',
    engine: 'Sous le capot',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Garde-fou du code écrit par IA · appris de votre historique git · 100 % local',
    titleLead: 'Lintez les règles',
    titleGradient: 'que personne n’a écrites.',
    subtitle:
      'L’IA écrit du code valide qui n’est pas [[le vôtre]] — et fait taire les tests qui le disent. argot apprend votre dépôt depuis son historique git et signale les deux, avant le merge.',
    ctaPrimary: 'Lire la doc',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · binaire statique unique · macOS · Linux · Windows · 100 % local',
    installAlt: 'ou installer sans npm',
  },
  demo: {
    label: 'La deuxième question',
    title: 'Le type-checker demande si ça compile. argot demande si c’est le vôtre.',
    body: 'Des PR propres et bien typées enterrent cette question. argot y répond au diff — [[voici sa vraie sortie]].',
    tabs: [
      {
        id: 'foreign-import',
        label: 'foreign-import',
        caption:
          'Du Python valide — mais un framework que ce dépôt n’a jamais importé. L’évidence montre ce qu’il utilise à la place.',
      },
      {
        id: 'redundant',
        label: 'redundant',
        caption: 'Le dépôt a déjà cette fonction. argot nomme l’originale et la similarité.',
      },
      {
        id: 'misplaced',
        label: 'misplaced',
        caption: 'Le bon code, au mauvais endroit — ses voisins vivent tous dans core/downloader.',
      },
      {
        id: 'layering',
        label: 'layering',
        caption: 'Ici, cli importe core — jamais l’inverse. Cet import retourne l’architecture.',
      },
      {
        id: 'test-disabled',
        label: 'test-disabled',
        caption: 'Vert parce que sauté, pas corrigé. argot nomme le test et le code qu’il couvre.',
      },
    ],
    seeLive: 'Voyez-le sur de vrais dépôts',
  },
  trust: {
    label: 'L’autre mode de défaillance',
    title: 'L’agent qui ne sait pas corriger le code « corrige » le test.',
    body: 'Le diff est propre, la CI repasse au vert — et votre filet de sécurité se troue [[exactement là où le code est le plus récent]]. argot associe le test affaibli au code qu’il couvre, et nomme les deux.',
    moves: [
      { name: 'le sauter', example: '@pytest.mark.skip("flaky")' },
      { name: 'le vider', example: 'assertions supprimées, test conservé' },
      { name: 'réajuster', example: 'attendu 429 → devient 200' },
      { name: 'le supprimer', example: 'test disparu, code conservé' },
    ],
    caption:
      '[[94 %]] des éditions truquées détectées · 0 des 102 refactorings légitimes signalés · sort en warn — informe, ne bloque jamais.',
  },
  audit: {
    label: 'Jour un',
    title: 'Auditez votre historique. Voyez ce que l’IA y a glissé.',
    body: '[[argot audit]] calibre la voix telle qu’elle était il y a 50 commits, rescore tout ce qui a suivi et attribue chaque signalement — [[ai-assisted, human ou unknown]] — depuis les marqueurs de commit, jamais le style. Une commande, zéro config, votre arbre intact.',
    caption:
      'Sur l’historique d’argot lui-même : [[52 %]] des commits portent des marqueurs IA — et l’unique signalement remonte à un commit assisté par IA.',
  },
  customRules: {
    label: 'Vos conventions',
    title: 'Le sixième détecteur, c’est vous.',
    body: 'Cinq détecteurs apprennent votre dépôt. Le sixième, vous l’écrivez : [[un manifeste et un petit script]] dans .argot/rules/, versionnés avec votre code, chargés à l’exécution. Les conventions qui vivaient en commentaires de review — appliquées à chaque diff, dans les 11 langages.',
    points: [
      {
        title: 'À l’échelle du diff',
        desc: 'Les règles ne voient que les fichiers modifiés — en adopter une crée [[zéro bruit d’historique]].',
      },
      {
        title: 'Consciente de l’historique',
        desc: 'import_attested("moment") demande [[« l’a-t-on déjà utilisé ? »]] — aucun autre linter ne le peut.',
      },
      {
        title: 'Pilotée par les tests',
        desc: 'argot rules test exécute vos fixtures — la [[boucle rouge/vert]] de l’auteur de règles.',
      },
    ],
    cta: 'Écrivez votre première règle',
  },
  engine: {
    label: 'Sous le capot',
    title: 'De la compréhension sémantique. Aucun LLM nulle part.',
    body: 'Quatre moteurs, un binaire [[Rust]] statique, tous appris de votre historique git — rien ne quitte votre machine.',
    cards: [
      {
        title: 'Un modèle d’embeddings de code sur votre laptop',
        desc: 'jina-code (~100 Mo, téléchargé une fois) transforme chaque fonction en vecteur — ainsi argot sait que vous [[l’avez déjà écrite]]. Un encodeur, pas un LLM.',
      },
      {
        title: 'Un modèle de voix statistique',
        desc: 'Deux tables de fréquences et une partition d’appels — les imports, les appels et les formes de tokens que votre dépôt [[utilise vraiment]].',
      },
      {
        title: 'Un graphe d’architecture',
        desc: 'La topologie de dépendances de vos modules. Une arête qui [[inverse la direction établie]] est signalée avec la direction qu’elle casse.',
      },
      {
        title: 'Un diff d’inventaire de tests',
        desc: 'tree-sitter suit ce que chaque test vérifie. Un test [[sauté, vidé ou supprimé]] à côté d’un changement de prod est associé et nommé.',
      },
    ],
    stats: [
      { value: '0,2 s', label: 'pour vérifier un diff' },
      { value: '0,6 s', label: 'quand il définit de nouvelles fonctions' },
      { value: '25 s', label: 'premier fit, dépôt de 1 100 fichiers' },
      { value: '4 s', label: 'pour rafraîchir — les embeddings sont réutilisés' },
    ],
    finePrint:
      'Mesuré sur FastAPI, CPU de portable. Un seul binaire statique — pas de Python, pas de Node.',
  },
  proof: {
    label: 'Mesuré, pas promis',
    title: 'Des chiffres honnêtes, sans fuite par construction.',
    stats: [
      {
        value: '98 %',
        title: 'motifs étrangers détectés',
        desc: '604 sur 618 — en ne se déclenchant que sur [[0,22 % des vraies modifications]].',
      },
      {
        value: '94 %',
        title: 'réinventions détectées · médiane',
        desc: 'Des réécritures des [[propres fonctions]] du dépôt, retracées à l’originale.',
      },
      {
        value: '96 %',
        title: 'mauvais placements détectés · médiane',
        desc: 'Là où le dépôt a une architecture séparable — il [[s’abstient]] là où il n’y en a pas.',
      },
      {
        value: '96,8 %',
        title: 'violations d’architecture détectées',
        desc: '244 inversions de layering sur 252, à [[zéro faux positif]] (0 sur 140 contrôles).',
      },
      {
        value: '94 %',
        title: 'trucages de tests détectés',
        desc: '144 sur 153 · 0 refactoring légitime signalé · [[1,24 %]] sur les commits acceptés réels.',
      },
    ],
    languages:
      'Un seul [[binaire statique]], 11 langages : Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP.',
    finePrint:
      'Rappel sur des motifs plantés dans de vrais fichiers ; fausses alertes par holdout temporel. Même l’angle mort structurel — l’étranger masqué — est publié, pas caché.',
    benchmarksCta: 'Tous les chiffres par dépôt →',
  },
  setup: {
    label: 'Configuration · conçu pour les agents',
    title: 'Un CLI que votre agent peut piloter.',
    body: 'Les skills apportent le jugement : [[/argot-setup]] lit votre dépôt, exclut ce qui ne doit pas façonner sa voix, calibre, et vérifie la détection.',
    installLabel: 'Ajoutez les skills — Claude Code, Cursor, 70+ agents',
    skillsIntro: 'cinq slash-commands que votre agent lance :',
    skillDescs: [
      'lit votre arbre, écrit argot.toml, vérifie la détection',
      'score chaque diff, signale l’étranger — ne bloque jamais',
      'examine une PR selon la voix de votre dépôt, sans checkout',
      'un score de voix non bloquant sur chaque PR',
      'transforme une convention d’équipe en règle testée',
    ],
    ctaLocal: 'Ou pilotez le CLI à la main',
    ctaCi: 'le guide CI',
    caption: 'Le modèle calibré reste hors de votre historique git.',
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
