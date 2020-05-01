def after_feature(context, feature):
    if context.players:
        for player_name, player in context.players.items():
            player.close()
