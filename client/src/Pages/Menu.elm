module Pages.Menu exposing (Model, Msg, gotEvent, init, toSession, update, view)

import Api exposing (ClientState(..), RoundId, ToBackend(..), ToClient(..), ToClientEnvelope(..), toClientDecoder)
import Html exposing (button, div, li, text, ul)
import Html.Events exposing (onClick)
import Http
import Session exposing (Session)


type alias Model =
    { session : Session
    , roundIds : List RoundId
    }


toSession : Model -> Session
toSession =
    .session


init sessionData =
    { session = { token = sessionData.token, username = "placeholder" }
    , roundIds = []
    }


type Msg
    = SendAction ToBackend
    | ActionSend (Result Http.Error ())
    | GotEvent ToClient


gotEvent : ToClient -> Msg
gotEvent =
    GotEvent


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SendAction toBackend ->
            ( model, Api.sendAction ActionSend model.session.token toBackend )

        GotEvent e ->
            ( fromBackend e model, Cmd.none )

        ActionSend _ ->
            ( model, Cmd.none )


fromBackend : ToClient -> Model -> Model
fromBackend toClient model =
    case toClient of
        AvailableRounds { roundIds } ->
            { model | roundIds = roundIds }

        _ ->
            model


view { roundIds } =
    let
        mkJoinRound roundId =
            li [] [ button [ onClick <| SendAction <| JoinGame roundId ] [ text <| "join " ++ roundId ] ]
    in
    div []
        [ text "menu"
        , button [ onClick <| SendAction StartGame ] [ text "start game" ]
        , button [ onClick <| SendAction GetAvailableRounds ] [ text "load rounds list" ]
        , ul [] <| List.map mkJoinRound roundIds
        ]
