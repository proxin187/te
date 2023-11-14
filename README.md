== TE - Manual ==

TE is short for text editor but it also means tea in norwegian.


Table of contents:
    - Building: How to build TE
        - Dependencies: Required software for TE to work
        - Instructions: Building manual

    - Editor Manual: Basic editor features and documentation 
        - Movement: Simple movement
        - Buffers: How to handle buffers

    - Configuration: How to setup language syntax


-> Building:
In order to build the TE text editor you will need the all the
dependencies specified below.

Dependencies:
    - cargo or rustc: To build TE

Instructions:
    - Step 1:
        To build TE you only need to run the build script with
        root permisions like shown below:
        [$ sudo ./build.sh]
    - Step 2:
        TE is now installed onto your system, please refer to
        the manual for instructions on how to use the editor.

-> Editor Manual:
TE is a vim like editor with most of its features stolen from
vim, actually its basicaly just a vim clone at this point.

Movement:
    - Simple: The main movement in TE is done with the arrows.
    - Jump By Word: The user can jump by words on the x axis,
                    using Shift+Right/Left
    - Jump By Paragraph: Just like with words you can jump,
                         one paragraph up or down on the
                         y axis using Shift+Up/Down

Buffers:
    - Changing: Moving between buffers can be done using
                Ctrl+Right/Left
    - Opening: To open a new buffer you can use the command
               ":O [FILENAME]"
    - Closing: TO close the current buffer you can use the command
               ":qb"


