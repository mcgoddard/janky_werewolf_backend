from behave import *
from player import Player

SOCKET_URL = "wss://0a4nr0hbsk.execute-api.eu-west-2.amazonaws.com/dev/"

@given('we connect {num_players} players')
def step_impl(context, num_players):
    players = {}
    messages = {}
    player_names = ["Adam", "Bob", "Charles", "Debbie", "Emma", "Fred", "George", "Harry"]
    for player_name in player_names[:int(num_players) - 1]:
        messages[player_name] = []
        players[player_name] = Player(player_name, messages[player_name], SOCKET_URL)
    context.players = players
    context.messages = messages

@given('use game state {state_name}')
def step_impl(context, state_name):
    assert True is not False

@when('we send {message_name} message from {player_name}')
def step_impl(context, message_name, player_name):
    assert True is not False

@then('{player_name} receives state {state_name}')
def step_impl(context, player_name, state_name):
    assert context.failed is False
