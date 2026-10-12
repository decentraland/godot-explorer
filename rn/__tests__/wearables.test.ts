import { CATEGORIES } from '../src/wearables';

describe('wearable catalog', () => {
  const items = CATEGORIES.flatMap(category => category.items);

  it('uses base-avatar urns', () => {
    expect(items.every(item => item.id.startsWith('urn:decentraland:off-chain:base-avatars:'))).toBe(true);
  });

  it('has unique ids and at least one item per category', () => {
    expect(new Set(items.map(item => item.id)).size).toBe(items.length);
    expect(CATEGORIES.every(category => category.items.length > 0)).toBe(true);
  });
});
