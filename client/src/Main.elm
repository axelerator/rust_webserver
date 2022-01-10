port module Main exposing (..)

import Api exposing (ClientState, ToBackend(..), ToClient(..), sendAction)
import Browser
import Html exposing (Html, div)
import Json.Decode as Decode
import Json.Encode exposing (Value)
import Pages.Login as Login exposing (Msg(..))
import Pages.Menu as Menu
import Pages.Round as Round
import Session exposing (Session)


port toClientEvent : (Value -> msg) -> Sub msg


port sseConnected : (Bool -> msg) -> Sub msg


port connectToSSE : String -> Cmd msg


main : Program () Model Msg
main =
    Browser.element
        { init = init
        , update = update
        , subscriptions = subscriptions
        , view = view
        }



-- MODEL


type Model
    = OnLogin Login.Model
    | OnRound Round.Model
    | OnMenu Menu.Model


sessionFromModel : Model -> Maybe Session
sessionFromModel model =
    case model of
        OnRound subModel ->
            Just <| Round.toSession subModel

        OnMenu subModel ->
            Just <| Menu.toSession subModel

        _ ->
            Nothing


init : () -> ( Model, Cmd Msg )
init _ =
    ( OnLogin (Login.init Nothing)
    , Cmd.none
    )



-- UPDATE


type Msg
    = ForLogin Login.Msg
    | ForRound Round.Msg
    | ForMenu Menu.Msg
    | Logout
    | SSEConnected
    | CouldNotSendAction
    | CouldNotDecodeEvent
    | ChangeToRound Session ClientState


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model ) of
        ( SSEConnected, _ ) ->
            ( model
            , case sessionFromModel model of
                Just session ->
                    sendAction (\_ -> CouldNotSendAction) session.token Init

                Nothing ->
                    Cmd.none
            )

        ( ChangeToRound session clientState, _ ) ->
            ( OnRound (Round.fromEnterRound session clientState), Cmd.none )

        ( ForLogin ((Login.GotLoginResponse httpResponse) as subMsg), OnLogin subModel ) ->
            let
                loginSuccessModel =
                    case httpResponse of
                        Ok loginResponse ->
                            case loginResponse of
                                Api.LoginSuccess sessionData ->
                                    Just
                                        ( OnMenu <| Menu.init sessionData
                                        , connectToSSE sessionData.token
                                        )

                                _ ->
                                    Nothing

                        _ ->
                            Nothing
            in
            case loginSuccessModel of
                Just ( chatModel, cmd ) ->
                    ( chatModel, cmd )

                Nothing ->
                    let
                        ( updateSubModel, cmd ) =
                            Login.update subMsg subModel
                    in
                    ( OnLogin updateSubModel
                    , Cmd.map ForLogin cmd
                    )

        ( ForLogin subMsg, OnLogin subModel ) ->
            let
                ( updateSubModel, cmd ) =
                    Login.update subMsg subModel
            in
            ( OnLogin updateSubModel
            , Cmd.map ForLogin cmd
            )

        ( Logout, _ ) ->
            ( OnLogin (Login.init <| Just "You got logged out")
            , connectToSSE ""
            )

        ( ForRound subMsg, OnRound subModel ) ->
            let
                ( updateSubModel, cmd ) =
                    Round.update subMsg subModel
            in
            ( OnRound updateSubModel
            , Cmd.map ForRound cmd
            )

        ( ForMenu subMsg, OnMenu subModel ) ->
            let
                ( updateSubModel, cmd ) =
                    Menu.update subMsg subModel
            in
            ( OnMenu updateSubModel
            , Cmd.map ForMenu cmd
            )

        _ ->
            ( model, Cmd.none )



-- SUBSCRIPTIONS


onEvent : Model -> Value -> Msg
onEvent model value =
    let
        decoderResult =
            Decode.decodeValue Api.eventDecoder value

        maybeSession =
            sessionFromModel model
    in
    case ( decoderResult, maybeSession ) of
        ( Ok (Api.AppMsg toClient), Just session ) ->
            case ( toClient, model ) of
                ( UpdateGameState { clientState }, _ ) ->
                    ChangeToRound session clientState

                ( EnterRound { clientState }, _ ) ->
                    ChangeToRound session clientState

                ( _, OnMenu _ ) ->
                    ForMenu <| Menu.gotEvent toClient

                ( _, OnRound _ ) ->
                    ForRound <| Round.gotEvent toClient

                _ ->
                    CouldNotDecodeEvent

        _ ->
            CouldNotDecodeEvent


subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch [ toClientEvent (onEvent model), sseConnected (\_ -> SSEConnected) ]



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ case model of
            OnLogin subModel ->
                Html.map ForLogin <| Login.view subModel

            OnRound subModel ->
                Html.map ForRound <| Round.view subModel

            OnMenu subModel ->
                Html.map ForMenu <| Menu.view subModel
        ]
