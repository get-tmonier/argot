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
    local: 'Local',
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
    body: 'Les linters et type-checkers répondent à « est-ce valide ? ». Ils ne savent pas répondre à « est-ce ainsi que cette équipe écrit ? ». Cela vivait dans la revue de code — jusqu’à ce qu’un LLM puisse l’enterrer sous cent PR propres et bien typées en une après-midi. [[argot est la couche qui repose la question.]]',
    codeTitle: 'routers/users.py',
    caption:
      'Décorateurs, Depends, le retour typé — tout est idiomatique en FastAPI. Le seul écart : un ValueError nu là où ce dépôt lève toujours HTTPException. mypy est content. Le linter n’a rien à dire. [[argot signale la ligne.]]',
  },
  catches: {
    label: 'Ce qu’il détecte',
    title: 'Techniquement correct. Socialement faux.',
    body: 'argot ne remplace ni ESLint, ni ruff, ni votre type-checker. Il attrape ce qu’ils ne savent pas formuler — les conventions que votre équipe a adoptées [[par répétition]], jamais par écrit.',
    items: [
      {
        title: 'Copier-coller de LLM',
        desc: 'Un bloc dont le style s’écarte nettement du fichier qui l’entoure — fluide dans la voix moyenne de tous les dépôts publics, pas la vôtre.',
      },
      {
        title: 'Une dépendance étrangère',
        desc: 'Un import — un paquet, un module, un header — que le dépôt n’a jamais utilisé. Le signal pour lequel argot est conçu, et celui qu’il détecte le plus fiablement.',
      },
      {
        title: 'Une API étrangère',
        desc: 'Un appel vers une bibliothèque dont le code s’écarte — un autre client HTTP, ORM ou logger que celui que le reste du dépôt emploie.',
      },
      {
        title: 'Anomalie stylistique',
        desc: 'Du code correct, typé, sans erreur de lint — mais qui ne ressemble à personne de cette équipe.',
      },
    ],
  },
  proof: {
    label: 'Mesuré, pas promis',
    title: 'Des chiffres honnêtes, sans fuite par construction.',
    stats: [
      {
        value: '121/121',
        title: 'ruptures « motif inédit » détectées',
        desc: 'Le critère pour lequel argot est conçu — un import, une API ou une dépendance étrangère que le dépôt n’a jamais utilisée — détecté à 100 % sur 16 corpus gatés, et 178/183 (97 %) sur l’ensemble des corpus catalogués. Inséré dans de vrais fichiers et jugé par le vrai pipeline fit → check. Les classes plus difficiles (nommage, sémantique) sont une couverture secondaire, publiée en rouge, jamais gatée.',
      },
      {
        value: '0,23 %',
        title: 'fausses alertes sur de vraies modifications',
        desc: 'Holdout temporel sur 27 dépôts — calibré sur un ancien commit, seuls des commits jamais vus rejoués, aucun train-on-test. C’est le taux d’over-fire : argot se déclenchant sur le code existant du dépôt lui-même. Chaque corpus est à ≤0,98 % ; les déclenchements sur une dépendance réellement nouvelle dans un vrai commit sont comptés à part comme détections (argot qui fait son travail), pas ici.',
      },
      {
        value: '150 ms',
        title: 'pour vérifier un changement',
        desc: 'Mesuré sur un espace de 34 000 fichiers, CPU de portable — assez rapide pour un hook pre-commit. Le fit unique qui apprend la voix du dépôt entier prend ~7 s. Pas de GPU, pas de cloud.',
      },
    ],
    finePrint:
      'Protocole sans fuite (issue #92) : rappel mesuré sur des fixtures plantées dans de vrais fichiers et jugées par le binaire livré ; fausses alertes par holdout temporel avec intervalles de confiance bootstrap au niveau commit. Méthodologie et résultats bruts par dépôt dans le journal de recherche public.',
  },
  local: {
    label: 'Comment il reste honnête',
    title: 'Deux tables de fréquence. Aucun réseau de neurones.',
    body: 'argot construit deux distributions de tokens — une depuis votre dépôt, une depuis une référence open-source générique — et signale les passages bien plus probables sous la générique. [[C’est tout le modèle.]] Il se calibre sur CPU en secondes et embarque son seuil par dépôt.',
    points: [
      {
        title: 'Rien ne quitte votre machine',
        desc: 'Aucun GPU, aucun cloud, aucune télémétrie. Le modèle, ce sont deux tables de fréquence et un log-ratio max — calibré en secondes, scoré en millisecondes.',
      },
      {
        title: 'Calibré par dépôt',
        desc: 'Le seuil est fixé à partir de votre propre code : « normal » veut dire normal ici — pas la moyenne de tous les dépôts publics d’un modèle.',
      },
      {
        title: 'Conscient du langage, pas verrouillé',
        desc: 'Un tokenizer tree-sitter analyse les passages partiels et invalides. Python et TypeScript d’emblée ; les monorepos mixtes ont un seuil par langage.',
      },
      {
        title: 'Des preuves, pas des impressions',
        desc: 'Chaque signalement nomme les tokens qui ont porté le score, leur fréquence dans votre dépôt, et le vocabulaire habituel ici à la place.',
      },
    ],
  },
  features: {
    label: 'Pourquoi argot',
    title: 'Se lit comme un linter. Pense comme un relecteur.',
    items: [
      {
        title: 'Un seul binaire Rust, rapide',
        desc: 'Un binaire unique lié statiquement — ni Python, ni Node, aucun runtime à installer, aucun modèle à télécharger. [[extract 5× plus rapide, check ~23× plus rapide]] que le moteur précédent, démarrage instantané et résultats identiques au bit près.',
      },
      {
        title: 'S’intègre à la CI',
        desc: 'argot check tourne à chaque commit, groupe les hits par fichier et [[sort en non-zéro]] dès qu’un passage s’écarte. Câblez-le comme ESLint.',
      },
      {
        title: 'Incrémental, pas une réécriture',
        desc: 'Pointez-le sur un dépôt, lancez extract → fit [[une fois]], puis check à l’infini. Aucune annotation, aucune config pour démarrer.',
      },
      {
        title: 'Preuves par passage',
        desc: 'Chaque hit montre les tokens fautifs et leur attestation — startedAt (0×) vs use (88×) — et le vocabulaire habituel du dépôt à la place.',
      },
      {
        title: 'Une sévérité réglable',
        desc: 'unusual · suspicious · foreign, relatifs au seuil calibré. Filtrez le bruit avec [[--min-severity]].',
      },
      {
        title: 'Calibration par langage',
        desc: 'Un monorepo Python + TypeScript reçoit un seuil par langage, dispatché par extension. Aucune distribution n’écrase l’autre.',
      },
      {
        title: 'Honnête sur lui-même',
        desc: 'Des benchmarks publics, un journal de recherche de 35 docs, et un avertissement [[linter probabiliste]] imprimé à chaque run. Vérifiez avant d’agir.',
      },
    ],
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
