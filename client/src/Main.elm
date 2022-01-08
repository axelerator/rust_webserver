port module Main exposing (..)

import Api
import Browser
import Html exposing (Html, button, div, input, span, text)
import Html.Events exposing (onClick)
import Json.Decode as Decode
import Json.Encode exposing (Value)
import Pages.Chat as Chat
import Pages.Login as Login
import Pages.Menu as Menu
import String exposing (fromInt)


port toClientEvent : (Value -> msg) -> Sub msg


port connectToSSE : String -> Cmd msg



-- MAIN


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
    | OnChat Chat.Model
    | OnMenu Menu.Model


init : () -> ( Model, Cmd Msg )
init _ =
    ( OnLogin (Login.init Nothing)
    , Cmd.none
    )



-- UPDATE


type Msg
    = ForLogin Login.Msg
    | ForChat Chat.Msg
    | ForMenu Menu.Msg
    | Logout


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model ) of
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

        ( ForChat subMsg, OnChat subModel ) ->
            let
                ( updateSubModel, cmd ) =
                    Chat.update subMsg subModel
            in
            ( OnChat updateSubModel
            , Cmd.map ForChat cmd
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


subscriptions : Model -> Sub Msg
subscriptions model =
    let
        msg jsonValue =
            case Chat.mapEvent jsonValue of
                Just chatEvent ->
                    ForChat chatEvent

                Nothing ->
                    Logout
    in
    case model of
        OnChat _ ->
            toClientEvent msg

        OnMenu _ ->
            let
                decode value =
                    case Decode.decodeValue Api.eventDecoder value of
                        Ok (Api.AppMsg event) ->
                            ForMenu <| Menu.gotEvent event

                        e ->
                            -- a bit extreme :-p needs proper error handling
                            let
                                _ =
                                    Debug.log "something went wrong " e
                            in
                            Logout
            in
            toClientEvent decode

        _ ->
            Sub.none



-- VIEW


view : Model -> Html Msg
view model =
    div []
        [ div [] [ text "LOOK MUM, NO SERVER!!8" ]
        , case model of
            OnLogin subModel ->
                Html.map ForLogin <| Login.view subModel

            OnChat subModel ->
                Html.map ForChat <| Chat.view subModel

            OnMenu subModel ->
                Html.map ForMenu <| Menu.view subModel
        ]
