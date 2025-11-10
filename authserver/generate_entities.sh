#!/bin/bash

sea-orm-cli generate entity -u sqlite://authserver.db?mode=rwc -l -o authserver_entity/src/