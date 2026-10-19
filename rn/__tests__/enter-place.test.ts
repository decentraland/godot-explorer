import { setCoreForTests } from '../src/core';
import { createMockCore } from '../src/core/mock';
import { Control, State } from '../src/core/native';
import { destinationOf, enterPlace } from '../src/world/enterPlace';

const enterWorld = jest.fn();
const control = jest.fn();
const MAIN = 'https://realm.test/main';

const place = (over = {}) => ({
  id: 'p',
  title: 'Plaza',
  creator: 'DCL',
  image: '',
  users: 1,
  likeRate: null,
  location: '10,20',
  realm: MAIN,
  x: 10,
  y: 20,
  ...over,
});

function withState(state: State) {
  const mock = createMockCore();
  setCoreForTests({
    ...mock,
    state: () => state,
    urls: () => ({ ...mock.urls(), mainRealm: MAIN }),
    enterWorld,
    control,
  });
}

beforeEach(() => {
  enterWorld.mockClear();
  control.mockClear();
});
afterEach(() => setCoreForTests(undefined));

describe('enterPlace', () => {
  it('enters the default realm without naming it', () => {
    withState(new State.SignedIn());
    enterPlace(place());
    expect(enterWorld).toHaveBeenCalledWith({
      realm: undefined,
      parcel: { x: 10, y: 20 },
    });
  });

  it('names a world realm', () => {
    withState(new State.SignedIn());
    enterPlace(place({ realm: 'https://w/foo.dcl.eth' }));
    expect(destinationOf(place({ realm: 'https://w/foo.dcl.eth' })).realm).toBe(
      'https://w/foo.dcl.eth',
    );
    expect(enterWorld).toHaveBeenCalledTimes(1);
  });

  it('teleports when the player is already in that realm', () => {
    withState(new State.InWorld({ realm: MAIN, parcel: { x: 0, y: 0 } }));
    enterPlace(place());
    expect(enterWorld).not.toHaveBeenCalled();
    expect(control).toHaveBeenCalledWith(
      new Control.Teleport({ parcel: { x: 10, y: 20 } }),
    );
  });

  it('replaces the destination from another realm or while loading', () => {
    withState(
      new State.InWorld({ realm: 'https://w/a', parcel: { x: 0, y: 0 } }),
    );
    enterPlace(place());
    expect(enterWorld).toHaveBeenCalledTimes(1);
    withState(
      new State.Loading({
        destination: { realm: MAIN, parcel: undefined },
        progress: 0.2,
      }),
    );
    enterPlace(place());
    expect(enterWorld).toHaveBeenCalledTimes(2);
    expect(control).not.toHaveBeenCalled();
  });
});
