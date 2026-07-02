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
        title: 'Dérive de convention',
        desc: 'Gestion d’erreurs, journalisation ou formes de contrôle de flux qui ne correspondent pas au reste du code.',
      },
      {
        title: 'Paradigme étranger',
        desc: 'De la POO à base de classes lâchée dans un code fonctionnel. Un def synchrone sur un chemin async critique. Le mauvais import.',
      },
      {
        title: 'Anomalie stylistique',
        desc: 'Du code correct, typé, sans erreur de lint — mais qui ne ressemble à personne de cette équipe.',
      },
    ],
  },
  proof: {
    label: 'Mesuré, pas promis',
    title: 'Les chiffres viennent du binaire livré.',
    stats: [
      {
        value: '93,6 %',
        title: 'des ruptures plantées attrapées',
        desc: '160 des 171 ruptures de paradigme construites à la main sur 10 dépôts open-source épinglés — jugées par le vrai pipeline fit → check, pas par un harnais de labo. Cinq dépôts à 100 %.',
      },
      {
        value: '0 %',
        title: 'de faux positifs sur 7 dépôts sur 10',
        desc: 'Les 30 derniers commits réels de chaque dépôt rejoués dans argot check. Sept reviennent sans aucun signalement ; aucun n’en dépasse une poignée.',
      },
      {
        value: '10/12',
        title: 'ruptures subtiles attrapées en production',
        desc: 'Des ruptures sans import — du snake_case dans un dépôt camelCase, des chaînes de promesses dans un code Effect, du DOM façon jQuery — plantées en direct dans un espace de 34 000 fichiers. Dix signalées.',
      },
      {
        value: '150 ms',
        title: 'pour vérifier un changement',
        desc: 'Mesuré sur un espace de 34 000 fichiers, CPU de portable — assez rapide pour un hook pre-commit. Le fit unique qui apprend la voix du dépôt entier prend ~7 s. Pas de GPU, pas de cloud.',
      },
    ],
    finePrint:
      'Benchmark en conditions réelles : chaque fixture est plantée dans son fichier hôte sur disque, stagée avec git, jugée par le binaire livré. Méthodologie et résultats bruts dans le journal de recherche public.',
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
