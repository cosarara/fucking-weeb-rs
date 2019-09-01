Fucking Weeb, in Rust
=====================

A library manager for animu (and TV shows, and whatever).
(A rewrite of <https://github.com/cosarara97/fucking-weeb>)

Why
---

I have my series split over different hard drives,
in nested directories with names like
"[Underwater] Something Something [Batch]".
That makes it hard to browse.

I then also have to remember what's the last episode I watched.
And if I'm watching that series with different
audio/subtitle settings than the default, change those.

Wouldn't it be cool if I could save all this information
in an easy to navigate library thingy? That's what this is.

![Screenshot](https://www.cosarara.me/jaume/images/fucking_weeb_screenshot.png)


Now go watch [the video].

## Extra Features

* Show posters
* Automatically find posters in [TMDb]
* Drag and drop posters from your browser and your file manager
* Did I say it displays posters?
* Set a video player command, which can be overriden
  per show
* It tries to get the show name from the directory name
* It follows the XDG standards with regard to config files and such

[the video]: http://www.cosarara.me/jaume/files/videos/fucking-weeb.webm
[TMDb]: https://www.themoviedb.org/

Running
-------

Install rust, cargo, and gtk3 then:

    $ cargo run


Progress
--------

Things I'm missing:

* Autoplay
* Auto filling in the Add form
* Importing databases from the original format
* Error dialogs

Why the rewrite?
----------------

I wasn't having enough fun with CHICKEN,
first of all because I'm a pretty bad scheme programmer,
but also because I faced a couple bugs it had
(which were later solved in version 4.12.0).

Anyway, who doesn't love to rewrite stuff?
The second time you write a program you do it better, and faster.

Rust is neat.

