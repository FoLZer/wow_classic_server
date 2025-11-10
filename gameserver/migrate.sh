#!/bin/bash

sea-orm-cli migrate -d gameserver_migration -u sqlite://gameserver.db?mode=rwc