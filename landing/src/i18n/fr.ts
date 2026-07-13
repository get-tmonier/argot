import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — détectez le code IA qui ne colle pas à votre dépôt',
    description:
      'argot est un garde-fou local pour le code écrit par IA. Il apprend les motifs de votre dépôt depuis son historique git, puis signale le code qui n’a pas sa place — une dépendance jamais utilisée, une fonction que vous avez déjà, du code au mauvais endroit, un import qui casse votre architecture. Et quand un agent fait taire un test en échec au lieu de le corriger — sauté, vidé ou supprimé juste à côté du code qu’il couvre — argot associe les deux et le dit. Une commande, argot audit, note votre historique récent sur un clone vierge et attribue chaque signalement à son commit : assisté par IA, humain ou inconnu. Propulsé par un modèle d’embeddings de code qui tourne sur votre machine. Sans LLM, sans cloud, sans GPU.',
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
    body: 'Un LLM enterre cette question sous des PR propres et bien typées. argot la pose au diff — [[voici sa vraie sortie]].',
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
      {
        id: 'test-disabled',
        label: 'test-disabled',
        caption:
          'Un test en échec passe au vert — parce qu’il a été sauté, pas corrigé. argot associe le test désactivé au changement de production qu’il couvre, et nomme les deux.',
      },
    ],
    seeLive: 'Voyez-le sur de vrais dépôts',
  },
  trust: {
    label: 'L’autre mode de défaillance',
    title: 'L’agent qui ne sait pas corriger le code « corrige » le test.',
    body: 'Un skip avec une raison plausible, l’assertion qui échoue supprimée, la valeur attendue réajustée, le fichier disparu — le diff est propre, la CI repasse au vert, et votre filet de sécurité se troue [[exactement là où le code est le plus récent]]. argot lit les deux côtés de chaque diff : dès qu’un test s’affaiblit dans le changement même qui touche le code qu’il couvre, il nomme le test et le fichier co-modifié. Votre historique lui apprend le va-et-vient normal des tests — les refactorings restent silencieux.',
    moves: [
      { name: 'le sauter', example: '@pytest.mark.skip("flaky")' },
      { name: 'le vider', example: 'assertions supprimées, test conservé' },
      { name: 'réajuster', example: 'attendu 429 → devient 200' },
      { name: 'le supprimer', example: 'test disparu, code conservé' },
    ],
    caption:
      'Mesuré comme le reste : [[94 %]] des éditions truquées détectées sur 22 dépôts / 11 langages · 0 des 102 refactorings légitimes signalés · 1,24 % de commits réels marqués. test-weakened sort en warn — argot informe, ne bloque jamais.',
  },
  audit: {
    label: 'Jour un',
    title: 'Auditez votre historique. Voyez ce que l’IA y a glissé.',
    body: 'On ne démontre pas un garde-fou sur le code qu’il vient d’apprendre — alors argot rembobine. [[argot audit]] calibre la voix telle qu’elle était il y a 50 commits, rescore tout ce qui a suivi et attribue chaque signalement à son commit d’origine — ai-assisted, human ou unknown, à partir de [[marqueurs de commit concrets]] uniquement, jamais du style. Une commande, zéro config, votre arbre intact.',
    caption:
      'Vraie exécution sur l’historique d’argot lui-même : [[52 %]] des commits portent des marqueurs IA, et l’unique signalement remonte à un commit assisté par IA — avec [[l’évidence du dépôt lui-même]].',
  },
  engine: {
    label: 'Sous le capot',
    title: 'De la compréhension sémantique. Aucun LLM nulle part.',
    body: 'Quatre moteurs derrière les cinq détecteurs, un seul binaire [[Rust]] statique, tous appris de votre historique git — pas de clé d’API, pas de GPU, rien ne quitte votre machine.',
    cards: [
      {
        title: 'Un modèle d’embeddings de code sur votre laptop',
        desc: 'jina-code (~100 Mo, téléchargé une fois) transforme chaque fonction en vecteur. C’est ainsi qu’argot sait que vous [[l’avez déjà écrite]] — et où elle a sa place. Un encodeur, pas un LLM : aucune génération, CPU d’abord, accéléré Metal sur Mac.',
      },
      {
        title: 'Un modèle de voix statistique',
        desc: 'Deux tables de fréquences et une partition d’appels, apprises de votre historique — les imports, les appels et les formes de tokens que votre dépôt [[utilise vraiment]]. Pas besoin de réseau de neurones pour savoir que django n’a rien à faire ici.',
      },
      {
        title: 'Un graphe d’architecture',
        desc: 'La topologie de dépendances de vos modules, calibrée depuis vos propres imports : quelles couches pointent vers lesquelles. Une nouvelle arête qui [[inverse la direction établie]] est signalée avec la direction qu’elle casse.',
      },
      {
        title: 'Un diff d’inventaire de tests',
        desc: 'tree-sitter analyse chaque fichier de test à chaque commit et suit ce que chacun vérifie. Quand un changement de code de production s’accompagne d’un test [[sauté, vidé ou supprimé]], argot associe les deux et nomme le test — aucun modèle, juste un diff structurel de la suite.',
      },
    ],
    stats: [
      { value: '0,2 s', label: 'pour vérifier un diff' },
      { value: '0,6 s', label: 'quand il définit de nouvelles fonctions' },
      { value: '25 s', label: 'premier fit, dépôt de 1 100 fichiers' },
      {
        value: '4 s',
        label: 'pour rafraîchir — les fonctions inchangées réutilisent leurs embeddings',
      },
      { value: '2,3 min', label: 'audit seedé d’un monorepo de 30k fonctions — contre 6,5' },
      { value: '2,7 min', label: 'refit à chaud de ce monorepo — contre 17, résultats identiques' },
    ],
    finePrint:
      'Mesuré sur FastAPI, CPU de portable. Un seul binaire statique — pas de Python, pas de Node, aucun runtime à installer. Un cache d’embeddings global à la machine et une calibration multi-cœur gardent les gros monorepos rapides — sans jamais changer un résultat.',
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
      {
        value: '94 %',
        title: 'trucages de tests détectés',
        desc: 'Sauter, vider ou supprimer un test pour faire passer au vert une suite en échec : 144 éditions truquées sur 153 détectées, 0 sur 102 refontes légitimes signalées — seulement [[1,24 % signalés sur les commits acceptés réels]].',
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
