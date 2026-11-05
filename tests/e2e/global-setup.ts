import { e2eHome, seedVerifiedTools } from "./environment";

/**
 * The shared E2E host starts with an empty app home on a new machine. The setup gate is
 * mandatory, so seed the two verified artifacts it expects. Tests that own their home and
 * exercise the install itself must not depend on this seed.
 */
export default async function globalSetup() {
  seedVerifiedTools(e2eHome);
}
