let initialLink: Promise<void> = Promise.resolve();
let previewLaunch = false;

/** Called once with the routing of the link that launched the app; resolves to whether it was a preview link. */
export function trackInitialLink(routing: Promise<boolean>) {
  initialLink = routing.then(preview => {
    previewLaunch = preview;
  });
}

/** Resolves once the link that launched the app, if any, has been routed. */
export const whenInitialLinkRouted = () => initialLink;

/** Whether a preview link launched the app. Handed out once, so a later sign-out does not repeat it. */
export function takePreviewLaunch(): boolean {
  const launched = previewLaunch;
  previewLaunch = false;
  return launched;
}
