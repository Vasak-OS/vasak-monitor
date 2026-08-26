# vasak-monitor

El monitor del sistema de VasakOS. Cinco pantallas: **Recursos**, **Aplicaciones**,
**Servicios**, **Limpieza** y **Registros**.

## La decisión más importante es cuándo *no* medir

Es la aplicación que muestra el consumo, así que no puede ser la primera de su
propia lista. Tres condiciones, las tres con test:

- **Con la ventana tapada no se mide.** Nadie mira números que se actualizan detrás
  de otra ventana, y medir ahí gasta exactamente lo que la pantalla dice cuidar.
- **En las pantallas que no cambian solas tampoco.** Servicios, Limpieza y Registros
  no se mueven por su cuenta; volver a consultarlos cada dos segundos es trabajo que
  nadie pidió.
- **No se lanza una consulta si la anterior no volvió.** `setInterval` no espera: con
  el backend lento, una respuesta vieja llega después de una nueva y pisa datos más
  recientes. Por eso el intervalo se reagenda con `setTimeout` desde que la medición
  **terminó**.

Y la interfaz lo dice. Sin eso, alguien que deja el monitor de fondo supone que
sigue gastando.

## Las cuatro trampas de `/proc`

Cada una tiene su test, y ninguna falla si se hace mal — todas informan un número
que parece razonable.

**La CPU sólo existe como diferencia.** `/proc/stat` da jiffies acumulados desde el
arranque; un porcentaje sobre una sola lectura es el promedio de toda la sesión, y
después de un rato encendido se queda clavado alrededor del 2%. Y `iowait` cuenta
como inactivo: esperando al disco la CPU no trabaja, y contarlo haría que copiar un
archivo grande pareciera un procesador saturado.

**La memoria se mide con `MemAvailable`, no con `MemFree`.** En Linux `MemFree`
tiende a cero porque el kernel usa lo que sobra como caché. En la máquina de
desarrollo son 300 MB de 16 GB: informarlo diría «te queda el 2%» estando holgada.
La caché se muestra aparte y explicada, para que nadie intente «liberar» algo que no
le está quitando nada.

**El disco muestra sólo lo que ocupa disco.** `/proc/mounts` lista decenas de
montajes que no son almacenamiento, y un dispositivo montado dos veces —subvolúmenes
de btrfs— es el mismo espacio. Los puntos de montaje vienen escapados en octal, así
que `Disco\040externo` hay que desescaparlo o la ruta que se consulta no existe.

**La red también es una diferencia**, y `lo` no cuenta: el tráfico local no sale de
la máquina, y contarlo hace que hablar con el llavero parezca actividad de red.

## Y dos de los procesos, que enseñó el `/proc` real

**El nombre en `stat` viene entre paréntesis y puede contener espacios y
paréntesis.** Partiendo la línea por espacios, todos los campos se corren y la CPU y
la memoria salen de otra columna.

**Chromium y Electron reescriben su `argv`** como un bloque con espacios en lugar de
nulos. Partiendo sólo por nulos apareció una fila llamada `app.asar
--enable-sandbox --ozone-platform=wayland …` de dos mil caracteres que rompía la
tabla. Eso no lo encontró un test escrito de antemano: lo encontró contrastar los
parsers con el `/proc` de la máquina, que es para lo que existe
`examples/sondeo.rs`.

Los procesos se agrupan por nombre porque un navegador son diez y ninguno es «el
consumo de Chrome». Con la agrupación arreglada, 208 procesos se resumen en 96
aplicaciones y Chrome aparece como una fila de 1,9 GB.

## Limpieza: la parte incómoda

Casi todo lo que promete «liberar memoria» en Linux **no libera nada**. Lo que
parece ocupado es caché de disco, y el sistema la devuelve sola cuando algo la pide.
Vaciarla obliga a volver a leer del disco lo que ya estaba en memoria: después va
más lento, no más rápido.

Así que acá no hay un botón que lo prometa. Hay dos que hacen algo medible, y dicen
qué hacen:

- **Devolver el swap a la memoria.** Con RAM libre, las páginas que quedaron en disco
  vuelven. En la máquina de desarrollo el swap estaba al 71% con la RAM al 46%, que
  es justo el caso donde sirve.
- **Vaciar la caché del kernel.** Ofrecida y explicada, para cuando se quiere medir
  algo desde frío. No como una mejora.

En disco, en cambio, hay espacio real: 10,8 GB en `~/.cache`, 5,3 GB de caché de
pacman, el diario y la papelera. Lo del usuario no pide autenticar; lo del sistema
pasa por `pkexec`, que lo pregunta con el agente de polkit que ya está corriendo.

## Cerrar una aplicación

`SIGTERM` al **grupo** de procesos, no `SIGKILL` al proceso. Lo primero le da al
programa la chance de guardar lo que tenga abierto, y matar sólo el padre deja los
hijos consumiendo lo mismo. La interfaz dice que *pide* cerrar y no que mata: quien
espera lo segundo y ve el programa abierto cree que falló.

## Servicios

Lo fallido primero, después lo de VasakOS, el resto alfabético. Un orden puramente
alfabético entierra un servicio caído en la mitad de cincuenta y cinco filas, que es
exactamente lo que alguien viene a buscar. Los del usuario se manejan directo; los
del sistema por `pkexec`, y la fila lo dice — si no, la contraseña aparece sin
explicación.

## Registros

`journalctl -o json` y no el formato tabulado: ese pierde el nivel de severidad y la
unidad, que son lo único que permite filtrar. Sin filtro, «los registros» son diez
mil líneas donde el error que se busca está en alguna parte.

Las horas se muestran en local. El diario informa en UTC, y sin convertir no
coinciden con nada de lo que la persona vio en el reloj del panel.

## Lo que falta

- **Gráficos con historia**, para ver una subida y no sólo el instante.
- **Uso de CPU por aplicación** como porcentaje comparable: hoy se informa el delta
  de jiffies, que sirve para ordenar pero no para leer como número.
- **Aviso cuando un servicio de VasakOS se cae**, sin tener que abrir esto.

## Licencia

GPL-3.0-or-later.
