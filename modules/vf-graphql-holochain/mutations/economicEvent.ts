/**
 * Economic event mutations
 *
 * @package: HoloREA
 * @since:   2019-05-27
 */

import { zomeFunction } from '../connection'
import {
  EconomicEventCreateParams,
  EconomicResourceCreateParams,
  EconomicEventResponse,
} from '@valueflows/vf-graphql'

// :TODO: how to inject DNA identifier?
const createEvent = zomeFunction('a1_observation', 'main', 'create_event')

// CREATE
interface CreateArgs {
  event: EconomicEventCreateParams,
  createResource: EconomicResourceCreateParams,
}
type createHandler = (root: any, args: CreateArgs) => Promise<EconomicEventResponse>

export const createEconomicEvent: createHandler = async (root, args) => {
  const { event, createResource } = args
  return (await createEvent)({ event, createResource })
}