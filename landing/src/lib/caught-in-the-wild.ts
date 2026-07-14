export interface LocalizedText {
  readonly en: string;
  readonly fr: string;
}

export interface CaughtCase {
  readonly id: string;
  readonly repo: string;
  readonly language: string;
  readonly rule: 'redundant' | 'foreign-import' | 'layering';
  readonly tier: 'foreign' | 'unusual';
  readonly attribution: 'ai-assisted' | 'human';
  readonly commitSha: string;
  readonly commitSubject: string;
  readonly path: string;
  readonly loc: string;
  readonly diff: string;
  readonly evidence: string;
  readonly story: LocalizedText;
  readonly whyNoLinter: LocalizedText;
  readonly upstreamUrl: string | null;
}

export const REPO_COUNT = 33;

export const CASES: readonly CaughtCase[] = [
  {
    id: 'dagster',
    repo: 'dagster',
    language: 'TypeScript',
    rule: 'redundant',
    tier: 'unusual',
    attribution: 'ai-assisted',
    commitSha: 'ecc8b1c23de5b0f987e3cb783599ea85638836b8',
    commitSubject:
      'Consolidate all the places we check asset wipe permissions to be consistent and correct',
    path: 'js_modules/ui-core/src/assets/useWipeMaterializations.tsx',
    loc: 'L9-L74',
    diff: `js_modules/ui-core/src/assets/useWipeDialog.tsx
+  const hasWipePermission =
+    unscopedPermissions.canWipeAssets?.enabled ||
+    (opts?.locationName && locationPermissions[opts.locationName]?.canWipeAssets?.enabled) ||
+    !!opts?.definitionHasWipePermission;
js_modules/ui-core/src/assets/useWipeMaterializations.tsx
+  const hasWipePermission = useMemo(() => {
+    if (unscopedPermissions.canWipeAssets?.enabled) {
+      return true;
+    }
+    return selected.every((a) => {
...
+      return !!a.definitionHasWipePermission;
+    });
+  }, [selected, unscopedPermissions, locationPermissions]);`,
    evidence: 'duplicates useWipeDialog (useWipeDialog.tsx:9) — similarity 0.86',
    story: {
      en: 'A commit titled "Consolidate all the places we check asset wipe permissions to be consistent and correct" deleted the one component that built this check and hand-wrote the same three-tier cascade — deployment-wide, then per-location, then per-asset — into two sibling hooks instead: a boolean chain in useWipeDialog.tsx, a .every() loop in useWipeMaterializations.tsx, with matching explanatory comments. argot flagged the second hook as a 0.86-similarity duplicate of the first. Both copies are still live at HEAD — the next permission-rule change now has to be made twice, the exact inconsistency the commit set out to fix. The commit carries a Co-authored-by: Claude trailer; nothing in the diff itself reads as anything other than a careful, deliberate rewrite.',
      fr: 'Un commit intitulé « Consolidate all the places we check asset wipe permissions to be consistent and correct » a supprimé l’unique composant qui construisait cette vérification et réécrit à la main la même cascade à trois niveaux — d’abord à l’échelle du déploiement, puis par emplacement, puis par asset — dans deux hooks voisins : une chaîne de booléens dans useWipeDialog.tsx, une boucle .every() dans useWipeMaterializations.tsx, commentaires explicatifs jumeaux inclus. argot a signalé le second hook comme un doublon du premier, similarité 0,86. Les deux copies sont toujours vivantes à HEAD — le prochain changement de règle de permission doit désormais être fait deux fois, exactement l’incohérence que le commit visait à corriger. Le commit porte un trailer Co-authored-by: Claude ; rien dans le diff lui-même ne trahit autre chose qu’une réécriture soignée et délibérée.',
    },
    whyNoLinter: {
      en: 'The two hooks live in different files with restructured logic and different signatures — no token-based clone detector matches them, and no type checker models "this repo already has a hook that does this."',
      fr: 'Les deux hooks sont dans des fichiers différents, avec une logique restructurée et des signatures différentes — aucun détecteur de clones par jeton, et aucun vérificateur de types ne modélise « ce dépôt a déjà un hook qui fait ça ».',
    },
    upstreamUrl: null,
  },
  {
    id: 'hono',
    repo: 'hono',
    language: 'TypeScript',
    rule: 'redundant',
    tier: 'unusual',
    attribution: 'human',
    commitSha: '5205e7c7cfdf9dfc2124244c1123ef4050983fd8',
    commitSubject: 'feat(aws-lambda): specify content-type as binary',
    path: 'src/adapter/aws-lambda/handler.ts',
    loc: 'L666-L670',
    diff: `-export const isContentTypeBinary = (contentType: string) => {
+export const defaultIsContentTypeBinary = (contentType: string): boolean => {
   return !/^(text\\/(plain|html|css|javascript|csv).*|application\\/(.*json|.*xml).*|image\\/svg\\+xml.*)$/.test(
     contentType
   )
 }

// Pre-existing twin, still at src/adapter/lambda-edge/handler.ts:214 today:
export const isContentTypeBinary = (contentType: string): boolean => {
  return !/^(text\\/(plain|html|css|javascript|csv).*|application\\/(.*json|.*xml).*|image\\/svg\\+xml.*)$/.test(
    contentType
  )
}`,
    evidence: 'duplicates isContentTypeBinary (lambda-edge/handler.ts:214) — similarity 0.86',
    story: {
      en: 'The aws-lambda adapter formalized defaultIsContentTypeBinary — a byte-identical twin of isContentTypeBinary, sitting one directory over in the lambda-edge adapter. argot flagged the pair at similarity 0.86. Duplication then did what duplication does: a later fix (#4469, "serve microsoft office files as binary") repaired the regex so Office-document MIME types stop matching the loose .*xml pattern — but only in the aws-lambda copy. At HEAD, lambda-edge still ships the old regex; the same corruption bug the fix closed for Lambda is still open for Lambda@Edge.',
      fr: 'L’adaptateur aws-lambda a formalisé defaultIsContentTypeBinary — un jumeau octet pour octet d’isContentTypeBinary, situé un dossier plus loin dans l’adaptateur lambda-edge. argot a signalé la paire à une similarité de 0,86. La duplication a ensuite fait ce que la duplication fait toujours : un correctif ultérieur (#4469, « serve microsoft office files as binary ») a réparé la regex pour que les types MIME des documents Office cessent de matcher le motif trop large .*xml — mais seulement dans la copie aws-lambda. À HEAD, lambda-edge sert toujours l’ancienne regex ; le même bug de corruption fermé pour Lambda reste ouvert pour Lambda@Edge.',
    },
    whyNoLinter: {
      en: 'Different files, different names, and after the later fix even different regex bodies — no clone detector keyed on tokens or names catches this.',
      fr: 'Fichiers différents, noms différents, et après le correctif même les corps de regex diffèrent — aucun détecteur de clones basé sur les tokens ou les noms ne l’attrape.',
    },
    upstreamUrl: null,
  },
  {
    id: 'rich',
    repo: 'rich',
    language: 'Python',
    rule: 'foreign-import',
    tier: 'foreign',
    attribution: 'human',
    commitSha: '72b0a9e964a32a9d65a9cf895f7758bb85e0c631',
    commitSubject: 'f string path',
    path: 'rich/_unicode_data/__init__.py',
    loc: 'L1-L7',
    diff: `+import bisect
+from importlib import import_module
...
+def _parse_version(version: str) -> tuple[int, int, int]:
...
+    while len(version_integers) < 3:
+        version_integers = (version_integers, 0)   # BUG: never grows -> infinite loop for e.g. "15.1"
...
+        if unicode_version in VERSION_SET:          # BUG: str tested against a frozenset of int-tuples, always False
+            version = unicode_version
+        else:
+            unicode_version_integers = _parse_version(unicode_version)
+            insert_position = bisect.bisect_left(
+                VERSION_ORDER, unicode_version_integers
+            )
+            version = VERSIONS[max(0, insert_position - 1)]`,
    evidence:
      'bisect — new to the repo (first import in 4,460 commits; cells.py and text.py still hand-roll it)',
    story: {
      en: 'Rich hand-rolls binary search — two of them, in cells.py and text.py, still in the tree at HEAD — and in six years of accepted history had never once imported bisect. A commit named "f string path" landed 9,500-plus lines of generated Unicode tables plus one hand-written loader, and argot flagged that loader for reaching into bisect: vocabulary genuinely foreign to the repo. The same hunk shipped with an infinite loop (a version-parsing loop that never terminates on input like "15.1") and a membership check comparing a string against a set of tuples, always false — both fixed quietly two days later in a commit titled "fix typing". The bug never reached a release; argot would have flagged that hunk at review time, before either fix was needed.',
      fr: 'Rich roule ses recherches binaires à la main — il y en a encore deux, dans cells.py et text.py, toujours dans l’arbre à HEAD — et en six ans d’historique accepté n’avait jamais importé bisect. Un commit nommé « f string path » a livré plus de 9 500 lignes de tables Unicode générées plus un loader écrit à la main, et argot a signalé ce loader pour son usage de bisect : un vocabulaire authentiquement étranger au dépôt. Le même hunk contenait une boucle infinie (une boucle de parsing de version qui ne termine jamais pour une entrée comme « 15.1 ») et un test d’appartenance comparant une chaîne à un ensemble de tuples, toujours faux — les deux corrigés discrètement deux jours plus tard dans un commit intitulé « fix typing ». Le bug n’a jamais atteint une release ; argot aurait signalé ce hunk au moment de la review, avant même qu’un correctif soit nécessaire.',
    },
    whyNoLinter: {
      en: 'bisect is ordinary stdlib — no linter flags importing it. Only a model of six years of this repo’s own vocabulary knows it has never been used here.',
      fr: 'bisect est un module stdlib banal — aucun linter ne signale son import. Seul un modèle du vocabulaire historique du dépôt sait qu’il n’a jamais été utilisé ici en six ans.',
    },
    upstreamUrl: null,
  },
  {
    id: 'saleor',
    repo: 'saleor',
    language: 'Python',
    rule: 'layering',
    tier: 'unusual',
    attribution: 'human',
    commitSha: 'e2ebabee9dcfb0cc25535a8dfea9a9fb1ab6b119',
    commitSubject: 'Gift cards as payment method',
    path: 'saleor/giftcard/gateway.py',
    loc: 'L1-L26',
    diff: `--- /dev/null
+++ b/saleor/giftcard/gateway.py
+from ..payment.interface import TransactionSessionData, TransactionSessionResult
+from ..payment.models import TransactionEvent, TransactionItem

--- a/saleor/payment/gateway.py
+from ..giftcard.const import GIFT_CARD_PAYMENT_GATEWAY_ID

--- a/saleor/payment/utils.py — deferred inside a function, same commit
+        from ..giftcard.gateway import (
+            transaction_initialize_session_with_gift_card_payment_method,
+        )`,
    evidence: 'payment already depends on giftcard — this import closes a cycle',
    story: {
      en: 'PR "Gift cards as payment method" wired a two-way dependency between saleor/payment and saleor/giftcard: the new giftcard/gateway.py imports payment’s interfaces and models, while payment/gateway.py now imports back from giftcard — and payment/utils.py hides one of those imports inside a function body to dodge the ImportError that would otherwise follow. The commit log confesses the fight: "Move some functions around to fix circular import issues," "Eradicate local imports." argot had learned payment → giftcard as this repo’s one-way direction from its history, and called the reversal on the commit that introduced it — reproduced end-to-end: fit at the parent commit, review the merge, four layering hits, exit 1.',
      fr: 'La PR « Gift cards as payment method » a soudé une dépendance bidirectionnelle entre saleor/payment et saleor/giftcard : le nouveau giftcard/gateway.py importe les interfaces et modèles de payment, tandis que payment/gateway.py importe désormais giftcard en retour — et payment/utils.py cache l’un de ces imports dans le corps d’une fonction pour esquiver l’ImportError qui en résulterait. Le message de commit avoue la lutte : « Move some functions around to fix circular import issues », « Eradicate local imports ». argot avait appris depuis l’historique du dépôt que payment → giftcard était le sens unique établi, et a signalé l’inversion sur le commit qui l’a introduite — reproduit de bout en bout : calibration au commit parent, review du merge, quatre signalements layering, exit 1.',
    },
    whyNoLinter: {
      en: 'The back-edge is deferred into a function body — the standard trick that defeats static import-cycle detection. Only a tool that has learned this repo’s own layering from its history can call the new edge a reversal.',
      fr: 'L’arête retour est différée dans le corps d’une fonction — l’astuce classique qui déjoue la détection statique de cycles d’import. Seul un outil qui a appris le sens de circulation propre à ce dépôt depuis son historique peut qualifier cette nouvelle arête d’inversion.',
    },
    upstreamUrl: null,
  },
  {
    id: 'faker',
    repo: 'faker',
    language: 'Python',
    rule: 'foreign-import',
    tier: 'foreign',
    attribution: 'human',
    commitSha: 'a1a1b2acb417c0f14d80292d6cfbf357041f93ee',
    commitSubject: 'feat: Add `uuid1` and `uuid7` providers to `misc` provider',
    path: 'faker/providers/misc/__init__.py',
    loc: 'L6-L12',
    diff: `@@ -6,6 +6,7 @@ import os
+import time
 import uuid
 import zipfile

@@ (uuid1)
+        # Use current time with random perturbation for the timestamp
+        # UUID1 timestamp is in 100-nanosecond intervals since 1582-10-15
+        nanoseconds = int(time.time() * 1e9)
+        # Add random perturbation to avoid collisions and ensure seedability affects the result
+        nanoseconds += self.generator.random.randint(0, 999999)

@@ (uuid7)
+        # 48-bit Unix timestamp in milliseconds
+        unix_ts_ms = int(time.time() * 1000) + self.generator.random.randint(0, 999)`,
    evidence:
      'time — new to the repo (zero import time / time.time() in faker/ before this commit)',
    story: {
      en: 'The new uuid1() and uuid7() providers reach for Python’s time module — never imported once anywhere in faker’s production code before this commit — to derive their UUID timestamps from the wall clock. That cuts against the one thing this library is for: reproducible randomness. uuid4(), defined twenty lines above, draws all 128 bits from the seeded generator and reproduces exactly under Faker.seed(); uuid1() and uuid7() now don’t. The commit message claims the change "ensures seedability"; a same-seed comparison says otherwise — and the new tests check types and versions, never a pinned seeded value, unlike the suite’s existing test_uuid4_seedability.',
      fr: 'Les nouveaux providers uuid1() et uuid7() font appel au module time de Python — jamais importé une seule fois dans le code de production de faker avant ce commit — pour dériver leurs timestamps depuis l’horloge murale. Cela va à l’encontre de la seule raison d’être de cette bibliothèque : un hasard reproductible. uuid4(), défini vingt lignes plus haut, tire ses 128 bits entièrement du générateur calibré par la seed et se reproduit exactement sous Faker.seed() ; uuid1() et uuid7() ne le font plus. Le message de commit affirme que le changement « ensures seedability » ; une comparaison à seed identique dit le contraire — et les nouveaux tests ne vérifient que les types et versions, jamais une valeur calibrée épinglée, contrairement à test_uuid4_seedability déjà présent dans la suite.',
    },
    whyNoLinter: {
      en: 'import time is perfectly idiomatic stdlib Python — no linter, type checker, or Semgrep rule flags it. Only 13 years of this repo’s own history shows it has never been used.',
      fr: 'import time est du Python stdlib parfaitement idiomatique — aucun linter, vérificateur de types ou règle Semgrep ne le signale. Seuls 13 ans d’historique propre à ce dépôt montrent qu’il n’a jamais été utilisé.',
    },
    upstreamUrl: null,
  },
];
