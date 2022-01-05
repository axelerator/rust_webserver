module Pages.Chat exposing (Model, Msg, fromTokenAndUsername, init, update, view)

import Html exposing (Html, div, text)
import Session exposing (Session)


type alias Model =
    { currentChannel : String
    , session : Session
    }


fromTokenAndUsername token username =
    { currentChannel = "a channel name"
    , session =
        { username = username
        , token = token
        }
    }


type Msg
    = NoOp


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    ( model, Cmd.none )


init =
    { currentChannel = "Home" }


view : Model -> Html Msg
view model =
    div [] [ text model.currentChannel ]
