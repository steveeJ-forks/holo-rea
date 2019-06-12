/**
 * Top-level queries relating to Economic Events
 *
 * @package: HoloREA
 * @since:   2019-05-27
 */

import { zomeFunction } from '../connection'

// :TODO: how to inject DNA identifier?
const readEvent = zomeFunction('a1_observation', 'main', 'get_event')

// Read a single event by ID
export const economicEvent = async (root, args) => {
  const { id } = args
  return (await readEvent)({ address: id })
}