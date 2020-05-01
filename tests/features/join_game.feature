Feature: start game

  Scenario: players join game with all roles
    Given we connect 8 players
    And use game state lobby
    When we send start_game_all_roles message from Adam
    Then Adam receives state fresh_game_all_roles
