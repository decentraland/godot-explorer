import { parsePreviewUrl, previewIdFromUrl } from '../src/gallery/url';

describe('previewIdFromUrl', () => {
  it('extracts the manifest id', () => {
    expect(previewIdFromUrl('dclrn://preview/backpack__all__1665-13665')).toBe(
      'backpack__all__1665-13665',
    );
  });

  it('ignores the query string and a trailing slash', () => {
    expect(previewIdFromUrl('dclrn://preview/a__b__1-2/?state=x')).toBe(
      'a__b__1-2',
    );
  });

  it('is null for the list, other hosts and other schemes', () => {
    expect(previewIdFromUrl('dclrn://preview')).toBeNull();
    expect(previewIdFromUrl('dclrn://preview/')).toBeNull();
    expect(previewIdFromUrl('dclrn://other/x')).toBeNull();
    expect(previewIdFromUrl('decentraland://preview/x')).toBeNull();
    expect(previewIdFromUrl('')).toBeNull();
  });
});

describe('parsePreviewUrl', () => {
  it('returns null for non-preview urls', () => {
    expect(parsePreviewUrl('decentraland://open')).toBeNull();
  });

  it('reads orientation and state overrides', () => {
    expect(
      parsePreviewUrl('dclrn://preview/x?orientation=landscape&state=empty'),
    ).toEqual({ id: 'x', orientation: 'landscape', state: 'empty' });
  });

  it('drops an invalid orientation and flags the list', () => {
    expect(parsePreviewUrl('dclrn://preview?orientation=diagonal')).toEqual({
      id: null,
    });
  });

  it('reads a web url with the id in the query', () => {
    expect(
      parsePreviewUrl(
        'http://localhost:8765/?preview=extra%2Fflow-account&orientation=landscape#x',
      ),
    ).toEqual({ id: 'extra/flow-account', orientation: 'landscape' });
    expect(parsePreviewUrl('http://localhost:8765/')).toBeNull();
  });
});
