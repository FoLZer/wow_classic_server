#!/bin/bash

sea-orm-cli generate entity -u sqlite://gameserver.db?mode=rwc -l -o gameserver_entity/src/ --entity-format dense