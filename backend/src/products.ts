// Source-of-truth for the credits granted per StoreKit product. Must stay in
// sync with `godot/src/iap/iap_manager.gd`'s _CREDITS_BY_PRODUCT.

export const CREDITS_BY_PRODUCT: Record<string, number> = {
  local_credits_10: 10,
  local_credits_50: 50,
  local_credits_100: 100,
};

export function creditsFor(productId: string): number {
  return CREDITS_BY_PRODUCT[productId] ?? 0;
}
