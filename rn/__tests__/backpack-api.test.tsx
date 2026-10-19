/// <reference types="node" />
import fs from 'fs';
import path from 'path';
import { HsvSliders } from '../src/ds';
import { act, create } from 'react-test-renderer';
import { ColorEditor } from '../src/backpack/ColorEditor';
import { getBackpackApi, setBackpackApi } from '../src/backpack/api';
import { setCoreForTests } from '../src/core';
import type { CoreApi } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { CameraFocus, Control } from '../src/core/native';
import { createMockApi } from '../src/backpack/mockApi';
import { liveApi } from '../src/backpack/liveApi';
import { LANDSCAPE, PORTRAIT } from '../src/backpack/gallerySpecs';

describe('api registry', () => {
  it('swaps the active api', () => {
    const original = getBackpackApi();
    const mock = createMockApi();
    setBackpackApi(mock);
    expect(getBackpackApi()).toBe(mock);
    setBackpackApi(original);
  });
});

describe('liveApi', () => {
  afterEach(() => {
    jest.restoreAllMocks();
    setCoreForTests(undefined);
  });

  it('forwards commands to the core', () => {
    const control = jest.fn();
    setCoreForTests({ ...createMockCore(), control } as CoreApi);
    const command = new Control.FocusCamera({ focus: CameraFocus.Head });
    liveApi.send(command);
    expect(control).toHaveBeenCalledWith(command);
  });

  it('rejects when the catalog cannot load and retries afterwards', async () => {
    const load = jest.fn().mockRejectedValue(new Error('offline'));
    setCoreForTests({
      ...createMockCore(),
      loadAvatarCatalog: load,
    } as CoreApi);
    await expect(liveApi.loadCatalog()).rejects.toThrow('offline');
    await expect(liveApi.loadCatalog()).rejects.toThrow('offline');
    expect(load).toHaveBeenCalledTimes(2);
  });
});

describe('ColorEditor', () => {
  it('coalesces slider changes and flushes the last one on unmount', () => {
    jest.useFakeTimers();
    const onChange = jest.fn();
    let tree!: ReturnType<typeof create>;
    act(() => {
      tree = create(
        <ColorEditor
          target="hair"
          value="#1c1c1c"
          onChange={onChange}
          onBack={() => {}}
        />,
      );
    });
    const sliders = tree.root.findByType(HsvSliders);
    act(() => {
      sliders.props.onChange('#aa0000');
      sliders.props.onChange('#bb0000');
    });
    expect(onChange).not.toHaveBeenCalled();
    act(() => jest.advanceTimersByTime(150));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('#bb0000');
    act(() => {
      sliders.props.onChange('#cc0000');
    });
    act(() => tree.unmount());
    expect(onChange).toHaveBeenLastCalledWith('#cc0000');
    jest.useRealTimers();
  });
});

describe('gallery specs', () => {
  it('covers every backpack manifest id exactly once', () => {
    const manifest = JSON.parse(
      fs.readFileSync(
        path.join(__dirname, '..', '..', 'docs', 'rn-ui-spec', 'manifest.json'),
        'utf8',
      ),
    ) as { screens: { id: string; seq: number }[] };
    const wanted = manifest.screens
      .filter(
        screen =>
          (screen.seq >= 104 && screen.seq <= 175) ||
          screen.seq === 304 ||
          screen.seq === 305,
      )
      .map(screen => screen.id)
      .sort();
    const ids = [...PORTRAIT, ...LANDSCAPE].map(row => row[0]);
    expect(ids.slice().sort()).toEqual(wanted);
  });
});
