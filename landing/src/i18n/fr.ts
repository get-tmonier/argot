import type { SiteContent } from './types';

const fr: SiteContent = {
  meta: {
    title: 'argot — trouvez le code que votre dépôt questionnerait',
    description:
      'Auditez votre historique, puis vérifiez les changements face aux usages déjà établis par votre dépôt. Argot présente des éléments pour le jugement humain.',
  },
  nav: {
    demo: 'Démo',
    audit: 'Audit',
    engine: 'Sous le capot',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Revue ancrée dans le dépôt · audit d’abord · jugement humain',
    titleLead: 'Trouvez le code que votre dépôt',
    titleGradient: 'questionnerait.',
    subtitle:
      'Commencez avec [[argot audit]] : la commande compare les changements acceptés à l’historique antérieur du dépôt, puis donne des éléments à examiner. Pour la récurrence, choisissez et configurez le chemin adapté à votre équipe.',
    ctaPrimary: 'Voir l’audit',
    ctaSecondary: 'Star sur GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote:
      'open source sous licence MIT · cœur local gratuit · aucun compte n’est jamais requis · audit et check s’exécutent localement',
    installAlt: 'ou installer sans npm',
    watchFilm: 'Voir le film',
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
    label: 'Commencez ici',
    title: 'Auditez les changements acceptés avant d’en faire une habitude.',
    body: '[[argot audit]] évalue le diff net base-vers-HEAD avec un fit historique. L’attribution par marqueurs de commit est un plancher, pas un recensement ; un signalement invite à examiner, il ne prouve jamais un défaut. Le reçu affiché est une [[fixture rédigée en deux commits]] dont la commande, la version, la sortie brute et le checksum sont commités.',
    caption:
      'Ensuite, [[argot init]] calibre le dépôt actuel et vous choisissez un chemin de vérification récurrent.',
  },
  customRules: {
    label: 'Vos conventions',
    title: 'Voyez vos conventions — puis imposez-les.',
    body: '[[argot conventions]] lit votre dépôt et vous montre ce qu’il fait déjà : son API partagée, et [[où vit chaque type de code]] — la validation dans les fichiers schema, l’accès base dans les migrations, la logique métier dans la couche service. Choisissez-en une et le sixième détecteur est à vous : un manifeste et un petit script dans .argot/rules/, n’importe quel langage, jusqu’aux [[.env qu’un linter n’ouvre jamais]].',
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
    title: 'De la compréhension sémantique. Aucun LLM génératif nulle part.',
    body: 'Quatre moteurs, un binaire [[Rust]] statique, tous appris de votre historique git — rien ne quitte votre machine.',
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
        value: '98 %',
        title: 'motifs étrangers détectés',
        desc: '595 sur 605 — en ne se déclenchant que sur [[0,29 % des vraies modifications]].',
      },
      {
        value: '89 %',
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
        desc: '244 inversions de layering sur 252, à [[zéro faux positif]] sur les contrôles (0 sur 140 · ≤2,7 % de sur-signalement sur l’historique rejoué).',
      },
      {
        value: '94 %',
        title: 'trucages de tests détectés',
        desc: '144 sur 153 · 0 refactoring légitime signalé · [[1,12 %]] des commits acceptés touchant aux tests, à sévérité bloquante.',
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
    label: 'Configuration · conçu pour les agents',
    title: 'De l’audit à une vérification récurrente que vous choisissez.',
    body: 'Lancez [[argot init]] pour calibrer le dépôt, puis choisissez une CLI/skill invoquée, un hook de commit configuré par l’utilisateur, ou une Action GitHub configurée dans le workflow. Le plugin Claude peut demander avant une écriture qui introduit une dépendance étrangère dans un dépôt calibré ; ce n’est pas une vérification complète à l’acceptation.',
    installLabel: 'Six skills à la demande pour les hôtes d’agents compatibles',
    skillsIntro: 'six slash-commands que votre agent lance :',
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
    label: 'En CI, sans la friction',
    title: 'Un signal PR ou push configuré dans le workflow.',
    body: 'L’Action GitHub est [[non bloquante par défaut]] ; un workflow peut choisir une barrière. Une divergence intentionnelle reste une décision humaine, et un mute devient une trace d’audit.',
    caption: 'Atterrit dans le résumé Actions, un commentaire de PR épinglé, et l’onglet Security.',
    badge:
      'Épinglez-le dans votre README — un [[badge en direct]] rafraîchi à chaque push. Chaque visiteur, chaque fork, voit que le dépôt parle toujours sa propre voix.',
  },
  cta: {
    title: 'Ajoutez la couche qui manque à votre CI.',
    body: 'Open source sous licence MIT. Auditez d’abord, calibrez ensuite un dépôt et choisissez le chemin de vérification à lancer.',
    primary: 'Commencer',
    secondary: 'Voir sur GitHub',
  },
  footer: {
    tagline: 'Un linter de voix pour les règles non écrites.',
    builtBy: 'Créé par Damien Meur',
    docs: 'Docs',
    npm: 'npm',
    privacy: 'Confidentialité',
  },
};

export default fr;
