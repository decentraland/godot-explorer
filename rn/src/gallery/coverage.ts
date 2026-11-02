export type ManifestEntry = {
  id: string;
  source: string;
  kind: string;
  seq: number;
  name: string;
  orient?: string;
};

const IGNORED_KINDS = ['superseded', 'strip', 'fragment'];

/** Manifest seqs the design reports call identical to another frame. */
export const DUPLICATE_SEQS = [
  18, 28, 29, 30, 37, 70, 79, 90, 182, 193, 194, 198, 199, 206, 248, 253, 280,
  297, 331,
];

export function coverage(
  manifest: ManifestEntry[],
  registered: string[],
  duplicates: number[] = DUPLICATE_SEQS,
) {
  const have = new Set(registered);
  const required = manifest.filter(
    e => !IGNORED_KINDS.includes(e.kind) && !duplicates.includes(e.seq),
  );
  const missing = required.filter(e => !have.has(e.id));
  return {
    required: required.length,
    covered: required.length - missing.length,
    missing,
  };
}
