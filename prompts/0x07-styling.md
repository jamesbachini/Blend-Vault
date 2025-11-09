I want to make the style of the frontend more closely match Blend Capital's app.

Please update the CSS to more closely match: https://mainnet.blend.capital

Use ./frontend/src/public/vault-icon.svg for the title logo in the top left.

Blend's site uses these colors or somewhere close to them:

The background is #191b1fff
Then they have menus and div container boxes in #212328ff
Within those div containers they have multiple boxes horizontally laid out which are #191b1fff
Highlights and selected elements are in #24382dff
Text is predominantly white, grey
Text on the highlight background is #32ac4aff

The text is:
    margin: 0px 0px 0px 6px;
    font-family: "DM Sans", Roboto;
    font-weight: 500;
    font-size: 1.125rem;
    line-height: 1.473;

Some other text on the page is:
    text-transform: none;
    font-family: "DM Sans", Roboto;
    font-weight: 500;
    font-size: 1rem;

I have copied some of the CSS code below:
color: #FFFFFF;
background-color: #191B1F;
background-color: #fff;
background-color: #212429E5;
color: #fff;
-webkit-transition: background-color 150ms cubic-bezier(0.4, 0, 0.2, 1) 0ms;
transition: background-color 150ms cubic-bezier(0.4, 0, 0.2, 1) 0ms;
background-color: rgba(255, 255, 255, 0.08);
transition: background-color 250ms cubic-bezier(0.4, 0, 0.2, 1) 0ms,box-shadow 250ms cubic-bezier(0.4, 0, 0.2, 1) 0ms,border-color 250ms cubic-bezier(0.4, 0, 0.2, 1) 0ms,color 250ms cubic-bezier(0.4, 0, 0.2, 1) 0ms;
background-color: #36B04A;
color: #36B04A;
color: #979797;
color: #FFCB00;
border-bottom-color: #FF3366;
background-color: #36B04A26;
background-color: rgba(25, 27, 31, 0.9);
color: #FF3366;
background: #36B04A26;
background: #212429E5;
background: #FFCB0026;
background: none;
background: #FF336626;