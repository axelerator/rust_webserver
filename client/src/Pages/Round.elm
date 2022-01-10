module Pages.Round exposing
    ( Model
    , Msg
    , fromEnterRound
    , gotEvent
    , toSession
    , update
    , view
    )

import Api exposing (ClientState(..), ToBackend(..), ToClient(..), ToClientEnvelope(..), sendAction)
import Html exposing (Html, button, div, li, p, text, ul)
import Html.Events exposing (onClick)
import Session exposing (Session)


type alias Model =
    { session : Session
    , events : List ToClient
    , clientState : Maybe ClientState
    }


toSession : Model -> Session
toSession =
    .session


fromEnterRound : Session -> ClientState -> Model
fromEnterRound session clientState =
    { session = session
    , events = []
    , clientState = Just clientState
    }


gotEvent : ToClient -> Msg
gotEvent =
    GotEvent


type Msg
    = NoOp
    | EventDecoderError String
    | GotEvent ToClient
    | SendAction ToBackend


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )

        EventDecoderError e ->
            ( Debug.log e model, Cmd.none )

        GotEvent e ->
            let
                newClientState =
                    case e of
                        UpdateGameState { clientState } ->
                            Just clientState

                        _ ->
                            model.clientState

                model_ =
                    { model
                        | events = e :: model.events
                        , clientState = newClientState
                    }
            in
            ( model_, Cmd.none )

        SendAction toBackend ->
            ( model, sendAction (\_ -> NoOp) model.session.token toBackend )


view : Model -> Html Msg
view model =
    div []
        --, ul [] <| List.map (\e -> li [] [ text <| eventToString e ]) model.events
        [ case model.clientState of
            Nothing ->
                text "waiting"

            Just state ->
                viewGame state
        ]


viewGame : ClientState -> Html Msg
viewGame client_state =
    case client_state of
        Lobby { playerCount, playerReadyCount } ->
            div []
                [ text "players "
                , text <| String.fromInt playerReadyCount
                , text " of "
                , text <| String.fromInt playerCount
                , text " are ready"
                , button [ onClick <| SendAction ToggleReady ] [ text "Ready" ]
                ]

        InLevel { currentInstruction, uiItems, instructionsExecuted } ->
            let
                mkUiItem { label, state, id } =
                    li []
                        [ text label
                        , text " is "
                        , button [ onClick <| SendAction <| ChangeSetting id ]
                            [ text <|
                                if state then
                                    "ON"

                                else
                                    "OFF"
                            ]
                        ]
            in
            div []
                [ p [] [ text "Instructions executed: ", text <| String.fromInt instructionsExecuted ]
                , p [] [ text "instruction:", text currentInstruction ]
                , ul [] <| List.map mkUiItem uiItems
                ]
