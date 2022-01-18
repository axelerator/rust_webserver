module Pages.Round exposing
    ( Model
    , Msg(..)
    , gotEvent
    , toSession
    , update
    , updateClientState
    , view
    )

import Api exposing (ClientState(..), ToBackend(..), ToClient(..), ToClientEnvelope(..), sendAction)
import Html.Styled exposing (Html, button, div, li, p, span, text, ul)
import Html.Styled.Attributes exposing (style)
import Html.Styled.Events exposing (onClick)
import Session exposing (Session)


type alias Model =
    { session : Session
    , events : List ToClient
    , clientState : Maybe ClientState
    , instructionOpacity : Float
    }


toSession : Model -> Session
toSession =
    .session


updateClientState : Session -> ClientState -> Maybe Model -> Model
updateClientState session clientState mbOldState =
    case ( clientState, mbOldState ) of
        ( InLevel newClientState, Just oldModel ) ->
            { session = session
            , events = []
            , clientState = Just clientState
            , instructionOpacity =
                case oldModel.clientState of
                    Just (InLevel cs) ->
                        if newClientState.currentInstruction == cs.currentInstruction then
                            oldModel.instructionOpacity

                        else
                            1.0

                    _ ->
                        1.0
            }

        _ ->
            { session = session
            , events = []
            , clientState = Just clientState
            , instructionOpacity = 1.0
            }


gotEvent : ToClient -> Msg
gotEvent =
    GotEvent


type Msg
    = NoOp
    | EventDecoderError String
    | GotEvent ToClient
    | SendAction ToBackend
    | Tick


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        NoOp ->
            ( model, Cmd.none )

        Tick ->
            ( { model | instructionOpacity = model.instructionOpacity - 0.2 }
            , Cmd.none
            )

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
                viewGame state model.instructionOpacity
        ]


viewGame : ClientState -> Float -> Html Msg
viewGame client_state opacity =
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

        InLevel { currentInstruction, uiItems, instructionsExecuted, instructionsMissed } ->
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
                , p [] [ text "Instructions missed: ", text <| String.fromInt instructionsMissed ]
                , p []
                    [ text "instruction:"
                    , span [ style "opacity" (String.fromFloat opacity) ]
                        [ text currentInstruction ]
                    ]
                , ul [] <| List.map mkUiItem uiItems
                ]
