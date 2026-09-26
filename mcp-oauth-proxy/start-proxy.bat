@echo off
setlocal
title FzComputerAI - MCP OAuth 2.1 Proxy
echo ========================================================
echo   FzComputerAI - MCP OAuth 2.1 Proxy Server
echo ========================================================
echo.

where python >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [ERRO] Python nao foi encontrado no PATH.
    echo Instale o Python 3.10+ em https://www.python.org/downloads/
    echo e marque a opcao "Add Python to PATH".
    echo.
    pause
    exit /b 1
)

echo Verificando dependencias Python...
python -m pip install -r "%~dp0requirements.txt" --quiet

echo.
echo Iniciando servidor proxy OAuth 2.1...
python "%~dp0server.py" %*

pause
