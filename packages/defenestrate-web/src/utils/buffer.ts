/** Buffer Utils */

/**
 * Converts a frame given by the emulator into an HTML5 ImageData object.
 *
 * This conversion is necessary because the emulator generates RGB8 buffers, whereas
 * the canvas uses RGBA8 buffers. The bit stride mismatch means we cannot directly
 * display the buffer onto the canvas, necessitating this conversion.
 *
 * TODO: Investigate if it's possible to use WebGL and display the buffer as a textured quad, and whether that
 * would yield any perf benefits.
 *
 * @param {ArrayBuffer} buffer The buffer in RGB8 to convert
 * @param {number} width The width of the buffer
 * @param {number} height The height of the buffer
 * @return {ImageData} An ImageData object in RGBA8 that can be directly blitted to a canvas
 */
export function convertEmuBufferToImageData(buffer: ArrayBuffer, width: number, height: number): ImageData {
    const imageData = new ImageData(width, height);
    const bufferView = new Uint8Array(buffer);
    const data = imageData.data;
    for (let row = 0; row < width; row++) {
        for (let col = 0; col < height; col++) {
            const idx = col * width + row;
            data[idx * 4 + 0] = bufferView[idx * 3 + 0];
            data[idx * 4 + 1] = bufferView[idx * 3 + 1];
            data[idx * 4 + 2] = bufferView[idx * 3 + 2];
            data[idx * 4 + 3] = 255; // opaque
        }
    }
    return imageData;
}
