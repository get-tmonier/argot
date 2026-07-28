import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — trouvez le code que votre dépôt questionnerait',
    description:
      'Un analyseur statistique de code construit à partir de l’historique de votre dépôt. Auditez les changements acceptés et examinez les éléments.',
  },
  nav: {
    demo: 'Démo',
    audit: 'Audit',
    engine: 'Sous le capot',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Vérifications ancrées dans le dépôt',
    titleLead: 'Un analyseur statistique de code',
    titleGradient: 'construit à partir de l’historique de votre dépôt.',
    subtitle: 'Auditez les changements acceptés. Examinez les éléments, puis décidez.',
    ctaPrimary: 'Commencer par un audit',
    ctaSetup: 'Configurez-le avec votre agent',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote:
      'open source sous licence MIT · cœur local gratuit · aucun compte n’est jamais requis · audit et check s’exécutent localement',
    installAlt: 'ou installer sans npm',
    watchFilm: 'Voir le film',
  },
  demo: {
    label: 'Un exemple concret',
    title: 'Le type-checker demande si ça compile. argot demande si c’est le vôtre.',
    body: 'Des PR propres et bien typées peuvent rester étrangères au dépôt. [[Voici sa vraie sortie.]]',
    tabs: [
      {
        id: 'foreign-import',
        label: 'foreign-import',
        caption:
          'Du Python valide — mais un framework que ce dépôt n’a jamais importé. L’évidence montre ce qu’il utilise à la place.',
      },
      {
        id: 'superseded',
        label: 'superseded',
        caption:
          'Ce dépôt a remplacé requests par httpx il y a des mois. argot cite les commits de la migration — et prévient avant un reste de plus.',
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
    label: 'Pourquoi cela compte',
    title: 'L’agent qui ne sait pas corriger le code « corrige » le test.',
    body: 'Un check vert peut masquer un test affaibli. argot l’associe au code modifié et nomme les deux.',
    moves: [
      { name: 'le sauter', example: '@pytest.mark.skip("flaky")' },
      { name: 'le vider', example: 'assertions supprimées, test conservé' },
      { name: 'réajuster', example: 'attendu 429 → devient 200' },
      { name: 'le supprimer', example: 'test disparu, code conservé' },
    ],
    caption:
      '[[93,9 %]] des éditions truquées détectées · 0 des 106 refactorings légitimes signalés · sort en warn — informe, ne bloque jamais.',
  },
  audit: {
    label: 'Une preuve reproductible',
    title: 'Auditez les changements acceptés avant d’en faire une habitude.',
    body: '[[argot audit]] compare les changements acceptés à l’historique qui les précède. Un signalement invite à examiner, il ne prouve pas un défaut.',
    caption: 'Ensuite, lancez [[argot init]] et choisissez un chemin de vérification récurrent.',
  },
  customRules: {
    label: 'Fonctionnalités avancées',
    title: 'Voyez vos conventions — puis imposez-les.',
    body: '[[argot conventions]] trouve l’API partagée et où vit le code. Transformez une convention en une petite règle testable.',
    points: [
      {
        title: 'Découvertes, pas devinées',
        desc: 'argot conventions liste ce que votre dépôt fait déjà — son API partagée, et [[où vit chaque type de code]] — pour qu’une règle parte de ce qu’argot a trouvé.',
      },
      {
        title: 'Des deux côtés du diff',
        desc: 'ts_query_old voit [[ce qu’un changement supprime]] — une règle qu’aucun linter classique ne peut exprimer.',
      },
      {
        title: 'Consciente de l’historique',
        desc: 'import_attested("moment") demande [[« l’a-t-on déjà utilisé ? »]] — aucun autre linter ne le peut.',
      },
      {
        title: 'Pilotée par les tests',
        desc: 'argot rules test exécute vos fixtures — la [[boucle rouge/vert]] de l’auteur de règles.',
      },
      {
        title: 'Inviolable',
        desc: 'locked = true fige une règle ; un diff qui la coupe, l’abaisse ou la réécrit [[déclenche rule-tampered]] — error épinglé, insuppressible, annotation PR bien visible.',
      },
    ],
    cta: 'Écrivez votre première règle',
  },
  engine: {
    label: 'Sous le capot',
    title: 'De la compréhension sémantique. Aucun LLM génératif dans le cœur.',
    body: 'Quatre moteurs locaux, un binaire [[Rust]] statique, tous ancrés dans votre historique git.',
    cards: [
      {
        title: 'Un modèle d’embeddings de code sur votre laptop',
        desc: 'jina-code (~100 Mo, téléchargé une fois) transforme chaque fonction en vecteur — ainsi argot sait que vous [[l’avez déjà écrite]]. Un encodeur, pas un LLM : le CPU suffit, un GPU (Metal sur Mac) accélère simplement.',
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
        value: '97,6 %',
        title: 'motifs étrangers détectés',
        desc: '622 sur 637 à travers 36 dépôts — en ne se déclenchant que sur [[0,60 % des vraies modifications]].',
      },
      {
        value: '89 %',
        title: 'réinventions détectées · médiane',
        desc: 'Des réécritures des [[propres fonctions]] du dépôt, retracées à l’originale.',
      },
      {
        value: '97 %',
        title: 'mauvais placements détectés · médiane',
        desc: 'Là où le dépôt a une architecture séparable — il [[s’abstient]] là où il n’y en a pas.',
      },
      {
        value: '97,1 %',
        title: 'violations d’architecture détectées',
        desc: '264 inversions de layering sur 272, à [[zéro faux positif]] sur les contrôles (0 sur 148 · ≤2,7 % de sur-signalement sur l’historique rejoué).',
      },
      {
        value: '93,9 %',
        title: 'trucages de tests détectés',
        desc: '154 sur 164 · 0 refactoring légitime signalé sur 106 · [[1,25 %]] des commits acceptés touchant aux tests, à sévérité bloquante.',
      },
    ],
    languages:
      'Un seul [[binaire statique]]. Douze langages — chacun avec son propre adapter tree-sitter et son propre modèle appris :',
    finePrint:
      'Rappel sur des motifs plantés dans de vrais fichiers ; fausses alertes par holdout temporel. Même l’angle mort structurel — l’étranger masqué — est publié, pas caché.',
    benchmarksCta: 'Tous les chiffres par dépôt →',
    caughtCta: 'À voir sur le vif →',
  },
  setup: {
    label: 'Comment ça marche',
    title: 'De l’audit à une vérification récurrente que vous choisissez.',
    body: 'Lancez [[argot init]], puis choisissez la CLI, les skills, un hook de commit ou une GitHub Action. Le plugin Claude ajoute une invite étroite avant écriture — pas une vérification complète à l’acceptation.',
    installLabel: 'Installer la CLI',
    skillsLabel: 'Ajouter les skills agent',
    skillsIntro: 'six skills à la demande pour les hôtes compatibles :',
    skillDescs: [
      'lit votre arbre, écrit argot.toml, vérifie la détection',
      'score chaque diff, signale l’étranger — ne bloque jamais',
      'examine une PR selon la voix de votre dépôt, sans checkout',
      'un score de voix non bloquant sur chaque PR',
      'transforme une convention énoncée en règle testée',
      'trouve vos conventions, en codifie une',
    ],
    pluginNote:
      'Le plugin Claude ajoute du contexte MCP optionnel et une invite pré-écriture étroite et fail-open ; l’agent choisit toujours quand appeler Argot.',
    pluginCta: 'Obtenir le plugin',
    ctaLocal: 'Ou pilotez le CLI à la main',
    ctaCi: 'le guide CI',
    caption: 'Le modèle calibré reste hors de votre historique git.',
  },
  ciScore: {
    label: 'Intégrations',
    title: 'Un signal PR ou push configuré dans le workflow.',
    body: 'L’Action GitHub est [[non bloquante par défaut]]. Une divergence intentionnelle reste une décision humaine, enregistrée comme piste d’audit.',
    caption: 'Atterrit dans le résumé Actions, un commentaire de PR épinglé, et l’onglet Security.',
    badge:
      'Épinglez-le dans votre README — un [[badge en direct]] rafraîchi à chaque push. Chaque visiteur, chaque fork, voit que le dépôt parle toujours sa propre voix.',
  },
  cta: {
    title: 'Ajoutez la couche qui manque à votre CI.',
    body: 'Open source sous licence MIT. Auditez d’abord, puis choisissez la vérification récurrente adaptée à votre workflow.',
    primary: 'Commencer',
    secondary: 'Voir sur GitHub',
  },
  footer: {
    tagline: 'Un analyseur statistique de code construit à partir de l’historique de votre dépôt.',
    builtBy: 'Créé par Damien Meur',
    docs: 'Docs',
    npm: 'npm',
    privacy: 'Confidentialité',
  },
};

export default fr;
