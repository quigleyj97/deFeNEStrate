const webpack = require('webpack');
const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');

const config = {
    entry: [
        'react-hot-loader/patch',
        './src/index.tsx'
    ],
    output: {
        path: path.resolve(__dirname, 'public/dist'),
        filename: 'bundle.js'
    },
    module: {
        rules: [
            {
                test: /\.(js|jsx)$/,
                use: 'babel-loader',
                exclude: /node_modules/
            },
            {
                test: /\.ts(x)?$/,
                loader: 'ts-loader',
                exclude: /node_modules/
            },
            {
                test: /\.scss$/,
                use: [
                    'style-loader',
                    'css-loader',
                    'sass-loader'
                ]
            }
        ]
    },
    devServer: {
        'static': {
            directory: './public/'
        }
    },
    resolve: {
        extensions: [
            '.tsx',
            '.ts',
            '.js'
        ],
        alias: {
            'react-dom': '@hot-loader/react-dom'
        }
    },
    plugins: [
        new WasmPackPlugin({
            crateDirectory: path.resolve("../defenestrate-core")
        })
    ],
    experiments: {
        asyncWebAssembly: true
    }
};

module.exports = config;
