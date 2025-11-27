#!/bin/bash

export DATABASE_URL=sqlite://gameserver.db?mode=rwc
sqlx migrate revert
sqlx migrate run