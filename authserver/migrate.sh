#!/bin/bash

sea-orm-cli migrate -d authserver_migration -u sqlite://authserver.db?mode=rwc