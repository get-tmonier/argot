export interface LocalizedText {
  readonly en: string;
  readonly fr: string;
}

export interface WildProofStatus {
  readonly verifiedCaseCount: 0;
  readonly inventoryUrl: string;
  readonly inventoryDate: string;
  readonly summary: LocalizedText;
}

// Kept for the dormant card component. No values of this type are published
// until a future case has a qualifying receipt.
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

// DR-10's mechanical outcome withheld every former story: none had a complete,
// public receipt. Keep this as data so a future verified case must provide a
// receipt rather than reintroducing a prose-only narrative.
export const WILD_PROOF_STATUS: WildProofStatus = {
  verifiedCaseCount: 0,
  inventoryUrl:
    'https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/caught-in-the-wild-inventory.md',
  inventoryDate: '2026-07-22',
  summary: {
    en: 'No historical wild-case story is displayed because the committed inventory found no qualifying public receipts.',
    fr: 'Aucun récit historique de cas réel n’est affiché, car l’inventaire versionné n’a trouvé aucun reçu public admissible.',
  },
};
