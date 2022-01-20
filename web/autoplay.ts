// check if the browser disallows autoplay without user interaction
window.supportsAutoplay = (async () => {
    await new Promise(resolve => setTimeout(resolve, 2500)); // don't use CPU during load

    // zero seconds, 1 sample, of silence
    // chrome has some issues with data URLs so we wrap it into an object URL
    const objURL = await fetch("data:audio/wave;base64,UklGRiYAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABAAZGF0YQIAAAAAAA==")
        .then(x => x.blob())
        .then(x => URL.createObjectURL(x));

    const audEle = document.createElement("audio");
    audEle.src = objURL;
    let supported = true;
    try {
        // ignore the warning sometimes generated by the below line
        await audEle.play();
    } catch (e) {
        supported = false;
    }
    audEle.pause();
    audEle.src = "about:blank";
    URL.revokeObjectURL(objURL);
    return supported;
})();
