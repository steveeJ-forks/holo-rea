/**
 * Top-level queries relating to Processes
 *
 * @package: HoloREA
 * @since:   2019-09-12
 */

import { DNAIdMappings } from '../types'
import { mapZomeFn } from '../connection'

import {
  Process,
} from '@valueflows/vf-graphql'

export default (dnaConfig: DNAIdMappings, conductorUri: string) => {
  const readOne = mapZomeFn(dnaConfig, conductorUri, 'observation', 'process', 'get_process')

  return {
    process: async (root, args): Promise<Process> => {
      return (await readOne({ address: args.id })).process
    },
  }
}
