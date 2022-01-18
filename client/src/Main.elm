port module Main exposing (..)

import Api exposing (ClientState(..), ToBackend(..), ToClient(..), sendAction)
import Browser
import Css exposing (Rem, Style, absolute, backgroundColor, hex, position, px, rem, top, transform, translateX, translateY, vh, vw, width)
import Css.Transitions exposing (easeIn, easeInOut, transition)
import Html.Styled exposing (Attribute, Html, div, text, toUnstyled)
import Html.Styled.Attributes exposing (css)
import Json.Decode as Decode
import Json.Encode exposing (Value)
import Pages.Login as Login exposing (Msg(..))
import Pages.Menu as Menu
import Pages.Round as Round
import Session exposing (Session)
import Time


port toClientEvent : (Value -> msg) -> Sub msg


port sseConnected : (Bool -> msg) -> Sub msg


port connectToSSE : String -> Cmd msg


main : Program () Model Msg
main =
    Browser.element
        { init = init
        , update = update
        , subscriptions = subscriptions
        , view = view >> toUnstyled
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

        ( ChangeToRound session clientState, OnRound roundModel ) ->
            ( OnRound (Round.updateClientState session clientState (Just roundModel)), Cmd.none )

        ( ChangeToRound session clientState, _ ) ->
            ( OnRound (Round.updateClientState session clientState Nothing), Cmd.none )

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
    let
        sseEventFromPort =
            toClientEvent (onEvent model)

        sseConnectedPort =
            sseConnected (\_ -> SSEConnected)

        tick =
            case model of
                OnRound { clientState } ->
                    case clientState of
                        Just (InLevel _) ->
                            Time.every 1000 (\_ -> ForRound Round.Tick)

                        _ ->
                            Sub.none

                _ ->
                    Sub.none
    in
    Sub.batch [ sseEventFromPort, sseConnectedPort, tick ]



-- VIEW


type Shade
    = S0
    | S1
    | S2
    | S3
    | S4
    | S5


type Col
    = Charocal
    | Aqua


toCol : ( Col, Shade ) -> Css.Color
toCol ( col, shade ) =
    case col of
        Charocal ->
            hex <|
                case shade of
                    S0 ->
                        "111111"

                    S1 ->
                        "333333"

                    S2 ->
                        "888888"

                    S3 ->
                        "BBBBBB"

                    S4 ->
                        "DDDDDD"

                    S5 ->
                        "F0F0F0"

        Aqua ->
            hex "3333DD"


type Dist
    = Tiny
    | Small
    | Medium
    | Big
    | Large


dist : Dist -> Rem
dist d =
    case d of
        Tiny ->
            rem 0.1

        Small ->
            rem 0.3

        Medium ->
            rem 0.5

        Big ->
            rem 0.8

        Large ->
            rem 1.0


padding : Dist -> Style
padding =
    Css.padding << dist


card : List (Attribute Msg) -> List (Html Msg) -> Html Msg
card attrs children =
    let
        css_ =
            css
                [ backgroundColor <| toCol ( Charocal, S5 )
                , padding Medium
                , width <| rem 30
                ]

        allAttrs =
            css_ :: attrs
    in
    div
        allAttrs
        children


view : Model -> Html Msg
view model =
    let
        menu m offScreen =
            card
                [ css
                    [ if offScreen then
                        transform <| translateX (vw -100)

                      else
                        transform <| translateX (vw 0)
                    , position absolute
                    , top <| dist Medium
                    , transition [ Css.Transitions.transform3 500 0 easeInOut ]
                    ]
                ]
                [ Html.Styled.map ForMenu <| Menu.view m ]

        login m offScreen =
            card
                [ css
                    [ if offScreen then
                        transform <| translateY (vh -100)

                      else
                        transform <| translateY (vh 0)
                    , transition [ Css.Transitions.transform3 500 0 easeInOut ]
                    ]
                ]
                [ Html.Styled.map ForLogin <| Login.view m ]
    in
    div []
        [ case model of
            OnLogin subModel ->
                login subModel False

            _ ->
                login (Login.init Nothing) True
        , case model of
            OnMenu subModel ->
                menu subModel False

            _ ->
                menu Menu.dummy True
        , case model of
            OnRound subModel ->
                Html.Styled.map ForRound <| Round.view subModel

            _ ->
                text ""
        ]



{-
   , case model of
       OnLogin subModel ->
           Html.map ForLogin <| Login.view subModel

       OnRound subModel ->
           Html.map ForRound <| Round.view subModel

       OnMenu subModel ->
           Html.map ForMenu <| Menu.view subModel
   ]
-}
